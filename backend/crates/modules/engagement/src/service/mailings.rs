//! **Écrire aux participants d'une édition, et garder ce qui est parti.**
//!
//! # Brouillon, puis envoi figé
//!
//! Un brouillon se modifie à volonté. L'envoyer fige ses destinataires
//! (`mailing_recipients`) dans la transaction qui le met en file : la liste qui
//! part est celle qu'on a vue, même si un inscrit arrive pendant l'envoi. Ce qui
//! est parti ne se modifie plus ; on le duplique.
//!
//! # Le droit se vérifie sur l'édition de l'envoi
//!
//! `engagement.mailing.send` sur la portée de l'édition, ET le périmètre
//! d'administration (règle métier n° 8). Une route qui vise un envoi par son
//! identifiant répond « introuvable » hors périmètre : un 403 dirait qu'il
//! existe.
//!
//! # Rien ne part dans la requête
//!
//! L'envoi et l'essai passent par la file « email » du worker, qui suit la
//! par lots, sans limite d'envoi (décision du commanditaire, 28/10).
//!
//! # Un envoi qui n'avance plus le dit (31/10)
//!
//! Le worker passe l'envoi en échec à son dernier essai. Au relevé (liste,
//! fiche), `engagement.fail_stalled_mailings()` rattrape celui qu'aucun travail
//! ne sert plus. Un envoi en échec se relance (ses seuls destinataires encore en
//! file), s'arrête ou se supprime.

use std::collections::HashMap;

use kernel::auth::{administered_events, has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::jobs::{self, NewJob};
use kernel::RequestContext;
use uuid::Uuid;

use crate::domain::dossier::MailingProposal;
use crate::domain::gabarits::GABARITS;
use crate::domain::mailing::{
    jetons_connus, normaliser_adresses, variable_connue, CreateMailingPayload, Mailing,
    MailingAddressSuggestion, MailingAudience, MailingCriterion, MailingListScreen, MailingPayload,
    MailingPreview, MailingTestResult, KINDS, VARIABLES, VARIABLES_DOSSIER,
};
use crate::domain::{render, sanitize};
use crate::jobs::send_mailing::{QUEUE, SEND_MAILING, SEND_MAILING_TEST};
use crate::realtime::{self, Auteur};
use crate::repo::{cross, mailings};
use crate::service::{mail_texts, mailing_dossier};
use crate::state::EngagementState;

/// Testée par permission, jamais par rôle, et toujours sur l'édition.
const PERMISSION: &str = "engagement.mailing.send";

/// Les bornes du modèle (`ck_mailings_copies`, `ck_mailings_extra`), rendues
/// sur le champ plutôt qu'en erreur de contrainte.
const COPIES_MAX: usize = 10;
const ADRESSES_MAX: usize = 500;

// -----------------------------------------------------------------------------
// Gardes
// -----------------------------------------------------------------------------

/// Les deux contrôles sur une édition nommée par l'écran.
pub(crate) async fn garder_edition(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<()> {
    if !cross::edition_existe(state.pool(), event_id).await? {
        return Err(ApiError::not_found());
    }
    if !administered_events(state.pool(), acteur)
        .await?
        .allows(event_id)
    {
        return Err(ApiError::not_found());
    }
    if !has_permission(state.pool(), acteur, PERMISSION, Scope::Event(event_id)).await? {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

/// Un envoi visé par son identifiant : hors droit, il n'existe pas.
async fn garder_envoi(
    state: &EngagementState,
    acteur: Uuid,
    id: Uuid,
) -> Result<mailings::LigneEnvoi> {
    let Some(envoi) = mailings::lire(state.pool(), id).await? else {
        return Err(ApiError::not_found());
    };
    if droit_sur_l_edition(state.pool(), acteur, envoi.event_id).await? {
        Ok(envoi)
    } else {
        Err(ApiError::not_found())
    }
}

/// Le droit d'écrire aux participants de l'édition : la garde d'un envoi, et
/// celle de ses sujets de temps réel (`realtime.rs`).
pub(crate) async fn droit_sur_l_edition(
    pool: &sqlx::PgPool,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<bool> {
    Ok(administered_events(pool, acteur).await?.allows(event_id)
        && has_permission(pool, acteur, PERMISSION, Scope::Event(event_id)).await?)
}

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

/// Le filet du relevé : n'ouvre une écriture que s'il y a un envoi en cours.
async fn verifier_les_envois_en_cours(state: &EngagementState, event_id: Uuid) -> Result<()> {
    if !mailings::a_des_envois_en_cours(state.pool(), event_id).await? {
        return Ok(());
    }
    let ctx = RequestContext::background("engagement.mailing_check");
    let mut tx = state.db().write(&ctx).await?;
    for id in mailings::basculer_les_bloques(&mut tx, event_id).await? {
        realtime::signaler(&mut tx, id, Auteur::Prevenu).await?;
    }
    tx.commit().await
}

pub async fn ecran(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<MailingListScreen> {
    garder_edition(state, acteur, event_id).await?;
    ecran_garde(state, event_id, None).await
}

/// L'écran, une fois l'édition gardée ; `proposal_id` le réduit à un dossier.
pub(crate) async fn ecran_garde(
    state: &EngagementState,
    event_id: Uuid,
    proposal_id: Option<Uuid>,
) -> Result<MailingListScreen> {
    verifier_les_envois_en_cours(state, event_id).await?;
    let team_cc = mailings::equipe_en_copie(state.pool()).await?;
    let (timezone, city) = cross::fuseau_edition(state.pool(), event_id).await?;
    let mut rows = mailings::lister(state.pool(), event_id, proposal_id).await?;
    let edition = mailings::titre_edition(state.pool(), event_id).await?;
    let mut dossiers: HashMap<Uuid, Option<MailingProposal>> = HashMap::new();
    for row in &mut rows {
        if !row.subject.contains("{{") {
            continue;
        }
        let dossier = match row.proposal_id {
            Some(id) => match dossiers.get(&id) {
                Some(d) => d.clone(),
                None => {
                    let d = mailing_dossier::dossier_de(state.pool(), Some(id)).await?;
                    dossiers.insert(id, d.clone());
                    d
                }
            },
            None => None,
        };
        row.shown_subject = objet_affiche(&row.subject, &edition, dossier.as_ref());
    }
    Ok(MailingListScreen {
        rows,
        team_cc,
        variables: VARIABLES.iter().map(|v| (*v).to_owned()).collect(),
        proposal_variables: VARIABLES_DOSSIER.iter().map(|v| (*v).to_owned()).collect(),
        timezone,
        city,
    })
}

pub async fn detail(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<Mailing> {
    let mut envoi = garder_envoi(state, acteur, id).await?;
    if envoi.status == "queued" {
        verifier_les_envois_en_cours(state, envoi.event_id).await?;
        envoi = mailings::lire(state.pool(), id)
            .await?
            .ok_or_else(ApiError::not_found)?;
    }
    assembler(state, envoi).await
}

async fn assembler(state: &EngagementState, e: mailings::LigneEnvoi) -> Result<Mailing> {
    let recipients = mailings::destinataires(state.pool(), e.id).await?;
    let attachments: Vec<_> = mailings::pieces_jointes(state.pool(), e.id)
        .await?
        .into_iter()
        .map(|p| p.fiche)
        .collect();
    let proposal = mailing_dossier::dossier_de(state.pool(), e.proposal_id).await?;
    let edition = mailings::titre_edition(state.pool(), e.event_id).await?;
    Ok(Mailing {
        shown_subject: objet_affiche(&e.subject, &edition, proposal.as_ref()),
        attachments_total_bytes: attachments.iter().map(|a| a.byte_size).sum(),
        attachments_max_bytes: mailings::poids_permis(state.pool()).await?,
        attachments,
        counts: mailings::compter(&recipients),
        recipients,
        id: e.id,
        event_id: e.event_id,
        status: e.status,
        subject: e.subject,
        body_html: e.body_html,
        audience: e.audience,
        extra_addresses: e.extra_addresses,
        cc: e.cc,
        bcc: e.bcc,
        reply_to: e.reply_to,
        template: e.habillage.template,
        variable_fallbacks: e.habillage.variable_fallbacks,
        reply_button: e.habillage.reply_button,
        cta_label: e.habillage.cta_label,
        cta_url: e.habillage.cta_url,
        mail_template_id: e.habillage.mail_template_id,
        mail_text_id: e.mail_text_id,
        proposal,
        duplicated_from: e.duplicated_from,
        created_by_name: e.created_by_name,
        queued_by_name: e.queued_by_name,
        queued_at: e.queued_at,
        completed_at: e.completed_at,
        cancelled_at: e.cancelled_at,
        failed_at: e.echec.failed_at,
        failure_reason: e.echec.failure_reason,
        relaunch_count: e.echec.relaunch_count,
        relaunched_at: e.echec.relaunched_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

fn objet_affiche(objet: &str, edition: &str, dossier: Option<&MailingProposal>) -> String {
    let mut valeurs = HashMap::from([("edition".to_owned(), edition.to_owned())]);
    if let Some(d) = dossier {
        valeurs.extend(d.values.clone());
    }
    render::remplir(objet, &valeurs)
}

/// La liste telle qu'elle partirait maintenant, avec le décompte et le nombre
/// des messages remis au serveur, copies comprises.
pub async fn audience(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<MailingAudience> {
    let envoi = garder_envoi(state, acteur, id).await?;
    let criteres = serde_json::to_value(&envoi.audience).map_err(ApiError::internal)?;
    let entries = mailings::audience(
        state.pool(),
        envoi.event_id,
        &criteres,
        &envoi.extra_addresses,
    )
    .await?;

    let total = entries.len() as i64;
    let suppressed = entries.iter().filter(|e| e.is_suppressed).count() as i64;
    let sendable = total - suppressed;
    let without_first_name = entries
        .iter()
        .filter(|e| !e.is_suppressed && e.first_name.is_none())
        .count() as i64;
    let par_message = 1 + envoi.cc.len() as i64 + envoi.bcc.len() as i64;
    let envelope_count = sendable * par_message;
    let without_account = entries
        .iter()
        .filter(|e| !e.is_suppressed && e.person_id.is_none())
        .count() as i64;

    // Qui recevra une valeur de secours : une variable citée, vide pour lui.
    let mut citees = render::variables_citees(&envoi.subject);
    citees.extend(render::variables_citees(&envoi.body_html));
    let manque = |e: &crate::domain::mailing::MailingAudienceEntry| {
        citees.iter().any(|v| match v.as_str() {
            "prenom" => e.first_name.is_none(),
            "nom" => e.last_name.is_none(),
            "organisation" => e.organization.is_none(),
            _ => false,
        })
    };
    let fallback_count = entries
        .iter()
        .filter(|e| !e.is_suppressed && manque(e))
        .count() as i64;

    Ok(MailingAudience {
        total,
        sendable,
        suppressed,
        without_first_name,
        without_account,
        fallback_count,
        envelope_count,
        suggested_mail_template_id: crate::repo::mail_templates::suggere(state.pool(), id).await?,
        entries,
    })
}

/// Le courriel tel qu'un destinataire le recevra, gabarit compris : rendu par
/// la même fonction que le worker. `email` absent : le premier destinataire
/// joignable, ou un exemple sans compte s'il n'y en a aucun.
pub async fn apercu(
    state: &EngagementState,
    acteur: Uuid,
    id: Uuid,
    email: Option<&str>,
) -> Result<MailingPreview> {
    let envoi = garder_envoi(state, acteur, id).await?;
    let criteres = serde_json::to_value(&envoi.audience).map_err(ApiError::internal)?;
    let entries = mailings::audience(
        state.pool(),
        envoi.event_id,
        &criteres,
        &envoi.extra_addresses,
    )
    .await?;
    let choisie = entries
        .iter()
        .filter(|e| !e.is_suppressed)
        .find(|e| email.is_none_or(|m| e.email.eq_ignore_ascii_case(m)));
    // Une variable du dossier sans valeur se dit ici, avant l'envoi.
    let dossier = mailing_dossier::exiger_valeurs(state.pool(), &envoi).await?;
    let edition = mailings::titre_edition(state.pool(), envoi.event_id).await?;
    let (adresse, mut variables) = match choisie {
        Some(e) => (
            e.email.clone(),
            serde_json::json!({
                "prenom": e.first_name, "nom": e.last_name,
                "organisation": e.organization, "email": e.email, "edition": edition,
            }),
        ),
        None => (
            "destinataire@exemple.org".to_owned(),
            serde_json::json!({ "email": "destinataire@exemple.org", "edition": edition }),
        ),
    };
    if let (Some(v), serde_json::Value::Object(d)) = (variables.as_object_mut(), &dossier) {
        v.extend(d.clone());
    }
    let valeurs = crate::domain::mailing::valeurs(&variables, &envoi.habillage.variable_fallbacks);
    let apparence = crate::jobs::send_mailing::apparence_de_lenvoi(
        state.db(),
        &envoi,
        &state.config().app_public_url,
    )
    .await?;
    // Les passages repris du dossier se voient dans l'aperçu, et là seulement.
    let message = crate::jobs::send_mailing::composer_surligne(
        "apercu".to_owned(),
        &adresse,
        &envoi,
        &valeurs,
        &apparence,
        &VARIABLES_DOSSIER,
    )?;
    Ok(MailingPreview {
        to: adresse,
        has_account: choisie.is_some_and(|e| e.person_id.is_some()),
        subject: message.subject,
        html: crate::domain::gabarits::pour_apercu(&message.html.unwrap_or_default()),
    })
}

// -----------------------------------------------------------------------------
// Brouillons
// -----------------------------------------------------------------------------

/// Un brouillon validé, prêt à écrire.
struct Brouillon {
    subject: String,
    body_html: String,
    audience: serde_json::Value,
    extra_addresses: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    reply_to: Option<String>,
    habillage: mailings::Habillage,
    mail_text_id: Option<Uuid>,
}

impl Brouillon {
    fn valeurs(&self) -> mailings::ValeursBrouillon<'_> {
        mailings::ValeursBrouillon {
            subject: &self.subject,
            body_html: &self.body_html,
            audience: self.audience.clone(),
            extra_addresses: &self.extra_addresses,
            cc: &self.cc,
            bcc: &self.bcc,
            reply_to: self.reply_to.as_deref(),
            habillage: &self.habillage,
            mail_text_id: self.mail_text_id,
        }
    }
}

fn habillage_par_defaut() -> mailings::Habillage {
    mailings::Habillage {
        template: "sobre".to_owned(),
        variable_fallbacks: serde_json::json!({ "prenom": "Madame, Monsieur" }),
        reply_button: false,
        cta_label: None,
        cta_url: None,
        mail_template_id: None,
    }
}

/// Le gabarit et ses options, fusionnés avec l'état connu.
fn preparer_habillage(
    base: mailings::Habillage,
    p: &MailingPayload,
) -> Result<mailings::Habillage> {
    let template = match &p.template {
        Some(t) if !GABARITS.contains(&t.as_str()) => {
            return Err(ApiError::validation(
                format!("Gabarit inconnu : « {t} »."),
                "template",
            ))
        }
        Some(t) => t.clone(),
        None => base.template,
    };
    let variable_fallbacks = match &p.variable_fallbacks {
        Some(secours) => {
            let mut objet = serde_json::Map::new();
            for (nom, valeur) in secours {
                if !VARIABLES.contains(&nom.as_str()) {
                    return Err(ApiError::validation(
                        format!("Variable inconnue : « {nom} »."),
                        "variable_fallbacks",
                    ));
                }
                let valeur = valeur.trim();
                if !valeur.is_empty() {
                    objet.insert(nom.clone(), serde_json::Value::String(valeur.to_owned()));
                }
            }
            serde_json::Value::Object(objet)
        }
        None => base.variable_fallbacks,
    };
    let vide = |v: &Option<String>| v.as_ref().map(|s| s.trim().to_owned());
    let (cta_label, cta_url) = match (vide(&p.cta_label), vide(&p.cta_url)) {
        (None, None) => (base.cta_label, base.cta_url),
        (texte, lien) => {
            let texte = texte.filter(|s| !s.is_empty());
            let lien = lien.filter(|s| !s.is_empty());
            if let Some(l) = &lien {
                if !(l.starts_with("https://") || l.starts_with("http://")) {
                    return Err(ApiError::validation(
                        "Le lien du bouton commence par https://.",
                        "cta_url",
                    ));
                }
            }
            // Les deux, ou aucun : un bouton sans lien ne mène nulle part.
            match (texte, lien) {
                (Some(t), Some(l)) => (Some(t), Some(l)),
                _ => (None, None),
            }
        }
    };
    // Une mise en page choisie seule retire le gabarit personnalisé ; le
    // gabarit nommé se vérifie à part (`resoudre_le_gabarit`), contre la base.
    let mail_template_id = match (&p.template, &p.mail_template_id) {
        (_, Some(_)) => base.mail_template_id,
        (Some(_), None) => None,
        (None, None) => base.mail_template_id,
    };
    Ok(mailings::Habillage {
        template,
        variable_fallbacks,
        reply_button: p.reply_button.unwrap_or(base.reply_button),
        cta_label,
        cta_url,
        mail_template_id,
    })
}

fn adresses(brutes: &[String], champ: &str, max: usize) -> Result<Vec<String>> {
    let liste = normaliser_adresses(brutes).map_err(|fautive| {
        ApiError::validation(format!("Adresse invalide : « {fautive} »."), champ)
    })?;
    if liste.len() > max {
        return Err(ApiError::validation(
            format!("{max} adresses au plus dans ce champ."),
            champ,
        ));
    }
    Ok(liste)
}

fn criteres_valides(audience: &[MailingCriterion]) -> Result<serde_json::Value> {
    if let Some(inconnu) = audience.iter().find(|c| !KINDS.contains(&c.kind.as_str())) {
        return Err(ApiError::validation(
            format!("Critère de destinataires inconnu : « {} ».", inconnu.kind),
            "audience",
        ));
    }
    serde_json::to_value(audience).map_err(ApiError::internal)
}

/// Fusionne une écriture partielle avec l'état connu, puis valide le tout.
fn preparer(base: Brouillon, p: &MailingPayload) -> Result<Brouillon> {
    let subject = p.subject.clone().unwrap_or(base.subject).trim().to_owned();
    if subject.chars().count() > 250 {
        return Err(ApiError::validation("250 caractères au plus.", "subject"));
    }
    let body_html = match &p.body_html {
        Some(brut) => sanitize::assainir(brut),
        None => base.body_html,
    };

    // Les variables inconnues ne bloquent pas l'enregistrement automatique (on
    // tape « {{pre » avant « {{prenom}} ») : l'écran les signale, l'envoi les
    // refuse.

    let audience = match &p.audience {
        Some(criteres) => criteres_valides(criteres)?,
        None => base.audience,
    };
    let extra_addresses = match &p.extra_addresses {
        Some(brutes) => adresses(brutes, "extra_addresses", ADRESSES_MAX)?,
        None => base.extra_addresses,
    };
    let cc = match &p.cc {
        Some(brutes) => adresses(brutes, "cc", COPIES_MAX)?,
        None => base.cc,
    };
    let bcc = match &p.bcc {
        Some(brutes) => adresses(brutes, "bcc", COPIES_MAX)?,
        None => base.bcc,
    };
    let reply_to = match &p.reply_to {
        Some(brute) if brute.trim().is_empty() => None,
        Some(brute) => {
            let liste = adresses(std::slice::from_ref(brute), "reply_to", 1)?;
            liste.into_iter().next()
        }
        None => base.reply_to,
    };

    let habillage = preparer_habillage(base.habillage, p)?;
    Ok(Brouillon {
        subject,
        body_html,
        audience,
        extra_addresses,
        cc,
        bcc,
        reply_to,
        habillage,
        mail_text_id: base.mail_text_id,
    })
}

/// Le texte type nommé, vérifié sur l'édition de l'envoi ; gardé d'avant, il
/// tombe si l'envoi change d'édition et qu'il n'y sert pas.
async fn resoudre_le_texte(
    state: &EngagementState,
    brouillon: &mut Brouillon,
    p: &MailingPayload,
    edition: Uuid,
) -> Result<()> {
    match p.mail_text_id.as_deref() {
        Some(brut) => {
            brouillon.mail_text_id =
                mail_texts::verifier_pour_edition(state, brut, edition).await?;
        }
        None => {
            if let Some(id) = brouillon.mail_text_id {
                if !mail_texts::sert_ledition(state, id, edition).await? {
                    brouillon.mail_text_id = None;
                }
            }
        }
    }
    Ok(())
}

/// Le gabarit personnalisé d'un brouillon : nommé, il doit exister et servir
/// l'édition de l'envoi (le sien ou un commun), et sa mise en page est
/// recopiée ; gardé d'avant, il tombe si l'envoi change d'édition.
async fn resoudre_le_gabarit(
    state: &EngagementState,
    brouillon: &mut Brouillon,
    p: &MailingPayload,
    edition: Uuid,
) -> Result<()> {
    let demande = match p.mail_template_id.as_deref().map(str::trim) {
        Some("") => {
            brouillon.habillage.mail_template_id = None;
            return Ok(());
        }
        Some(brut) => Some(
            Uuid::parse_str(brut)
                .map_err(|_| ApiError::validation("Gabarit inconnu.", "mail_template_id"))?,
        ),
        None => brouillon.habillage.mail_template_id,
    };
    let Some(id) = demande else {
        return Ok(());
    };
    match crate::repo::mail_templates::lire(state.pool(), id).await? {
        Some(g) if g.event_id.is_none_or(|e| e == edition) => {
            brouillon.habillage.mail_template_id = Some(g.id);
            brouillon.habillage.template = g.layout;
        }
        // Un gabarit d'une autre édition ne suit pas l'envoi qui en change.
        Some(_) if p.mail_template_id.is_none() => brouillon.habillage.mail_template_id = None,
        _ => {
            return Err(ApiError::validation(
                "Ce gabarit n'existe pas ou ne sert pas cette édition.",
                "mail_template_id",
            ))
        }
    }
    Ok(())
}

fn brouillon_de(e: &mailings::LigneEnvoi) -> Result<Brouillon> {
    Ok(Brouillon {
        subject: e.subject.clone(),
        body_html: e.body_html.clone(),
        audience: serde_json::to_value(&e.audience).map_err(ApiError::internal)?,
        extra_addresses: e.extra_addresses.clone(),
        cc: e.cc.clone(),
        bcc: e.bcc.clone(),
        reply_to: e.reply_to.clone(),
        habillage: e.habillage.clone(),
        mail_text_id: e.mail_text_id,
    })
}

/// Crée un brouillon. Sans copie précisée, la copie de l'équipe est proposée :
/// les adresses réglées, plus celle de la personne qui écrit, pour qu'elle voie
/// les réponses de ceux qui « répondent à tous ».
pub async fn creer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &CreateMailingPayload,
) -> Result<Mailing> {
    garder_edition(state, acteur, payload.event_id).await?;
    creer_garde(
        state,
        ctx,
        acteur,
        payload.event_id,
        &payload.draft,
        &[],
        None,
    )
    .await
}

/// La création, une fois l'édition gardée. `cc_en_plus` s'ajoute à la copie
/// de l'équipe proposée par défaut ; `proposal_id` rattache le brouillon à un
/// dossier de cette édition.
pub(crate) async fn creer_garde(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    event_id: Uuid,
    draft: &MailingPayload,
    cc_en_plus: &[String],
    proposal_id: Option<Uuid>,
) -> Result<Mailing> {
    let mut cc_par_defaut = mailings::equipe_en_copie(state.pool()).await?;
    if let Some((adresse, ..)) = mailings::adresse_de(state.pool(), acteur).await? {
        cc_par_defaut.push(adresse);
    }
    cc_par_defaut.extend(cc_en_plus.iter().cloned());
    let base = Brouillon {
        subject: String::new(),
        body_html: String::new(),
        audience: serde_json::json!([]),
        extra_addresses: Vec::new(),
        cc: normaliser_adresses(&cc_par_defaut).unwrap_or_default(),
        bcc: Vec::new(),
        reply_to: None,
        habillage: habillage_par_defaut(),
        mail_text_id: None,
    };
    let mut brouillon = preparer(base, draft)?;
    resoudre_le_gabarit(state, &mut brouillon, draft, event_id).await?;
    resoudre_le_texte(state, &mut brouillon, draft, event_id).await?;

    let mut tx = state.db().write(ctx).await?;
    let id = mailings::creer(
        &mut tx,
        event_id,
        acteur,
        &brouillon.valeurs(),
        None,
        proposal_id,
    )
    .await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn modifier(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
    payload: &MailingPayload,
) -> Result<Mailing> {
    let envoi = garder_envoi(state, acteur, id).await?;
    if envoi.status != "draft" {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    }
    let mut brouillon = preparer(brouillon_de(&envoi)?, payload)?;
    // Changer d'édition : le droit se vérifie aussi sur la nouvelle. Un
    // courriel de dossier reste dans l'édition du dossier.
    let edition = match payload.event_id {
        Some(nouvelle) if nouvelle != envoi.event_id => {
            if envoi.proposal_id.is_some() {
                return Err(ApiError::validation(
                    "Ce courriel est écrit depuis un dossier : il reste dans l'édition du dossier.",
                    "event_id",
                ));
            }
            garder_edition(state, acteur, nouvelle).await?;
            nouvelle
        }
        _ => envoi.event_id,
    };
    resoudre_le_gabarit(state, &mut brouillon, payload, edition).await?;
    resoudre_le_texte(state, &mut brouillon, payload, edition).await?;

    let mut tx = state.db().write(ctx).await?;
    if !mailings::modifier(&mut tx, id, edition, &brouillon.valeurs()).await? {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    }
    tx.commit().await?;
    detail(state, acteur, id).await
}

/// Un brouillon, un envoi terminé, arrêté ou en échec disparaissent, destinataires et
/// rattachements compris. Un envoi en cours se refuse : le travail d'envoi le
/// sert encore, on l'arrête d'abord.
pub async fn supprimer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<()> {
    garder_envoi(state, acteur, id).await?;
    let mut tx = state.db().write(ctx).await?;
    // Avant le `DELETE` : la ligne porte l'édition à prévenir.
    realtime::signaler(&mut tx, id, Auteur::Tenu).await?;
    if !mailings::supprimer(&mut tx, id).await? {
        // Supprimé entre-temps par quelqu'un d'autre : il n'existe plus.
        return Err(match mailings::lire(state.pool(), id).await? {
            Some(_) => ApiError::new(ErrorCode::EngagementMailingInProgress),
            None => ApiError::not_found(),
        });
    }
    tx.commit().await
}

/// Au plus huit personnes à proposer pour un champ d'adresses.
const SUGGESTIONS_MAX: i64 = 8;

/// Les personnes connues de la plateforme dont le nom ou l'adresse
/// correspondent à la saisie. Gardée comme l'historique : qui peut écrire aux
/// participants de l'édition. Moins de deux caractères : rien.
pub async fn suggestions(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
    saisie: &str,
) -> Result<Vec<MailingAddressSuggestion>> {
    garder_edition(state, acteur, event_id).await?;
    let mots = mots_cherches(saisie);
    if mots.is_empty() {
        return Ok(Vec::new());
    }
    Ok(
        cross::personnes_a_proposer(state.pool(), &mots, SUGGESTIONS_MAX)
            .await?
            .into_iter()
            .map(|(name, email)| MailingAddressSuggestion { name, email })
            .collect(),
    )
}

/// Les mots d'une saisie, jokers de `LIKE` échappés. Vide sous deux
/// caractères ; bornée à cinq mots de quarante caractères.
fn mots_cherches(saisie: &str) -> Vec<String> {
    let saisie = saisie.trim();
    if saisie.chars().count() < 2 {
        return Vec::new();
    }
    saisie
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|m| !m.is_empty())
        .take(5)
        .map(|m| {
            m.chars()
                .take(40)
                .flat_map(|c| match c {
                    '\\' | '%' | '_' => vec!['\\', c],
                    _ => vec![c],
                })
                .collect()
        })
        .collect()
}

/// Un nouveau brouillon reprenant tout d'un envoi passé : objet, corps,
/// critères, adresses, copies. Les critères sont réévalués à l'envoi.
pub async fn dupliquer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<Mailing> {
    let envoi = garder_envoi(state, acteur, id).await?;
    let brouillon = brouillon_de(&envoi)?;
    let mut tx = state.db().write(ctx).await?;
    let nouveau = mailings::creer(
        &mut tx,
        envoi.event_id,
        acteur,
        &brouillon.valeurs(),
        Some(id),
        envoi.proposal_id,
    )
    .await?;
    tx.commit().await?;
    detail(state, acteur, nouveau).await
}

// -----------------------------------------------------------------------------
// Envoi
// -----------------------------------------------------------------------------

fn exiger_un_contenu(e: &mailings::LigneEnvoi) -> Result<()> {
    if e.subject.trim().is_empty() {
        return Err(ApiError::validation("L'objet est vide.", "subject"));
    }
    if sanitize_vide(&e.body_html) {
        return Err(ApiError::validation("Le message est vide.", "body_html"));
    }
    let mut citees = render::variables_citees(&e.subject);
    citees.extend(render::variables_citees(&e.body_html));
    if let Some(inconnue) = citees.iter().find(|v| !variable_connue(v)) {
        return Err(ApiError::validation(
            format!(
                "Variable inconnue : {{{{{inconnue}}}}}. Possibles : {}.",
                jetons_connus()
            ),
            "body_html",
        ));
    }
    Ok(())
}

/// Une pièce jointe en cours d'analyse ou écartée ne part pas : l'envoi attend
/// qu'elle soit prête plutôt que de partir sans elle.
async fn exiger_des_pieces_pretes(state: &EngagementState, id: Uuid) -> Result<()> {
    let pieces = mailings::pieces_jointes(state.pool(), id).await?;
    if let Some(p) = pieces.iter().find(|p| p.fiche.status != "ready") {
        return Err(ApiError::validation(
            format!(
                "La pièce jointe « {} » n'est pas prête (analyse en cours, ou fichier refusé) : réessayez dans un instant, ou retirez-la.",
                p.fiche.filename
            ),
            "attachments",
        ));
    }
    Ok(())
}

fn sanitize_vide(html: &str) -> bool {
    crate::domain::mailing::texte_brut(html).trim().is_empty()
}

/// L'essai part à la personne qui écrit, sans copie ni copie cachée, avec ses
/// propres valeurs de variables. Il n'est pas compté dans l'historique.
pub async fn essai(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<MailingTestResult> {
    let envoi = garder_envoi(state, acteur, id).await?;
    exiger_un_contenu(&envoi)?;
    mailing_dossier::exiger_valeurs(state.pool(), &envoi).await?;
    exiger_des_pieces_pretes(state, id).await?;
    let Some((adresse, ..)) = mailings::adresse_de(state.pool(), acteur).await? else {
        return Err(ApiError::not_found());
    };

    let mut tx = state.db().write(ctx).await?;
    jobs::enqueue(
        &mut tx,
        NewJob {
            queue: QUEUE,
            task: SEND_MAILING_TEST,
            payload: serde_json::json!({ "mailing_id": id, "person_id": acteur }),
            idempotency_key: None,
            run_at: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(MailingTestResult { to: adresse })
}

/// Fige les destinataires et met l'envoi en file, dans une transaction.
pub async fn envoyer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<Mailing> {
    let envoi = garder_envoi(state, acteur, id).await?;
    if envoi.status != "draft" {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    }
    exiger_un_contenu(&envoi)?;
    let dossier = mailing_dossier::exiger_valeurs(state.pool(), &envoi).await?;
    exiger_des_pieces_pretes(state, id).await?;

    let mut tx = state.db().write(ctx).await?;
    let Some(partiront) = mailings::figer(&mut tx, id, acteur, &dossier).await? else {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    };
    if partiront == 0 {
        // La transaction tombe avec l'erreur : le brouillon reste un brouillon.
        return Err(ApiError::new(ErrorCode::EngagementMailingEmpty));
    }
    jobs::enqueue(
        &mut tx,
        NewJob {
            queue: QUEUE,
            task: SEND_MAILING,
            payload: serde_json::json!({ "mailing_id": id }),
            idempotency_key: Some(format!("mailing:{id}:start")),
            run_at: None,
        },
    )
    .await?;
    realtime::signaler(&mut tx, id, Auteur::Tenu).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

/// Arrête un envoi en cours ou en échec : ce qui est parti reste parti, le
/// reste ne part pas.
pub async fn annuler(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<Mailing> {
    garder_envoi(state, acteur, id).await?;
    let mut tx = state.db().write(ctx).await?;
    if !mailings::annuler(&mut tx, id).await? {
        return Err(ApiError::validation(
            "Seul un envoi en cours ou en échec peut être arrêté.",
            "status",
        ));
    }
    realtime::signaler(&mut tx, id, Auteur::Tenu).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

/// Relance un envoi en échec : seuls ses destinataires encore en file partent.
/// Ceux qui sont partis ne reçoivent rien de plus ; ceux en échec ou au départ
/// non confirmé non plus (un doublon parti ne se rattrape pas).
pub async fn relancer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<Mailing> {
    let envoi = garder_envoi(state, acteur, id).await?;
    if envoi.status != "failed" {
        return Err(ApiError::validation(
            "Seul un envoi en échec peut être relancé.",
            "status",
        ));
    }
    exiger_des_pieces_pretes(state, id).await?;

    let mut tx = state.db().write(ctx).await?;
    let Some(numero) = mailings::relancer(&mut tx, id).await? else {
        return Err(ApiError::validation(
            "Plus aucun destinataire n'attend : rien à relancer.",
            "status",
        ));
    };
    jobs::enqueue(
        &mut tx,
        NewJob {
            queue: QUEUE,
            task: SEND_MAILING,
            payload: serde_json::json!({ "mailing_id": id }),
            idempotency_key: Some(format!("mailing:{id}:relance:{numero}")),
            run_at: None,
        },
    )
    .await?;
    realtime::signaler(&mut tx, id, Auteur::Tenu).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}
