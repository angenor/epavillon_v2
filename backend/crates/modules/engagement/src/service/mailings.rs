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

use kernel::auth::{administered_events, has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::jobs::{self, NewJob};
use kernel::RequestContext;
use uuid::Uuid;

use crate::domain::gabarits::GABARITS;
use crate::domain::mailing::{
    normaliser_adresses, CreateMailingPayload, Mailing, MailingAudience, MailingCriterion,
    MailingListScreen, MailingPayload, MailingPreview, MailingTestResult, KINDS, VARIABLES,
};
use crate::domain::{render, sanitize};
use crate::jobs::send_mailing::{QUEUE, SEND_MAILING, SEND_MAILING_TEST};
use crate::repo::{cross, mailings};
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
async fn garder_edition(state: &EngagementState, acteur: Uuid, event_id: Uuid) -> Result<()> {
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
    let perimetre = administered_events(state.pool(), acteur).await?;
    let autorise = has_permission(
        state.pool(),
        acteur,
        PERMISSION,
        Scope::Event(envoi.event_id),
    )
    .await?;
    if perimetre.allows(envoi.event_id) && autorise {
        Ok(envoi)
    } else {
        Err(ApiError::not_found())
    }
}

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

pub async fn ecran(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<MailingListScreen> {
    garder_edition(state, acteur, event_id).await?;
    let team_cc = mailings::equipe_en_copie(state.pool()).await?;
    Ok(MailingListScreen {
        rows: mailings::lister(state.pool(), event_id).await?,
        team_cc,
        variables: VARIABLES.iter().map(|v| (*v).to_owned()).collect(),
    })
}

pub async fn detail(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<Mailing> {
    let envoi = garder_envoi(state, acteur, id).await?;
    assembler(state, envoi).await
}

async fn assembler(state: &EngagementState, e: mailings::LigneEnvoi) -> Result<Mailing> {
    let recipients = mailings::destinataires(state.pool(), e.id).await?;
    let attachments: Vec<_> = mailings::pieces_jointes(state.pool(), e.id)
        .await?
        .into_iter()
        .map(|p| p.fiche)
        .collect();
    Ok(Mailing {
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
        duplicated_from: e.duplicated_from,
        created_by_name: e.created_by_name,
        queued_by_name: e.queued_by_name,
        queued_at: e.queued_at,
        completed_at: e.completed_at,
        cancelled_at: e.cancelled_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
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
    let edition = mailings::titre_edition(state.pool(), envoi.event_id).await?;
    let (adresse, variables) = match choisie {
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
    let valeurs = crate::domain::mailing::valeurs(&variables, &envoi.habillage.variable_fallbacks);
    let message = crate::jobs::send_mailing::composer(
        "apercu".to_owned(),
        &adresse,
        &envoi,
        &valeurs,
        &state.config().app_public_url,
    )?;
    Ok(MailingPreview {
        to: adresse,
        has_account: choisie.is_some_and(|e| e.person_id.is_some()),
        subject: message.subject,
        html: message.html.unwrap_or_default(),
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
    Ok(mailings::Habillage {
        template,
        variable_fallbacks,
        reply_button: p.reply_button.unwrap_or(base.reply_button),
        cta_label,
        cta_url,
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
    })
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

    let mut cc_par_defaut = mailings::equipe_en_copie(state.pool()).await?;
    if let Some((adresse, ..)) = mailings::adresse_de(state.pool(), acteur).await? {
        cc_par_defaut.push(adresse);
    }
    let base = Brouillon {
        subject: String::new(),
        body_html: String::new(),
        audience: serde_json::json!([]),
        extra_addresses: Vec::new(),
        cc: normaliser_adresses(&cc_par_defaut).unwrap_or_default(),
        bcc: Vec::new(),
        reply_to: None,
        habillage: habillage_par_defaut(),
    };
    let brouillon = preparer(base, &payload.draft)?;

    let mut tx = state.db().write(ctx).await?;
    let id = mailings::creer(
        &mut tx,
        payload.event_id,
        acteur,
        &brouillon.valeurs(),
        None,
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
    let brouillon = preparer(brouillon_de(&envoi)?, payload)?;
    // Changer d'édition : le droit se vérifie aussi sur la nouvelle.
    let edition = match payload.event_id {
        Some(nouvelle) if nouvelle != envoi.event_id => {
            garder_edition(state, acteur, nouvelle).await?;
            nouvelle
        }
        _ => envoi.event_id,
    };

    let mut tx = state.db().write(ctx).await?;
    if !mailings::modifier(&mut tx, id, edition, &brouillon.valeurs()).await? {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    }
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn supprimer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<()> {
    garder_envoi(state, acteur, id).await?;
    let mut tx = state.db().write(ctx).await?;
    if !mailings::supprimer_brouillon(&mut tx, id).await? {
        return Err(ApiError::new(ErrorCode::EngagementMailingNotDraft));
    }
    tx.commit().await
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
    if let Some(inconnue) = citees.iter().find(|v| !VARIABLES.contains(&v.as_str())) {
        return Err(ApiError::validation(
            format!(
                "Variable inconnue : {{{{{inconnue}}}}}. Possibles : {}.",
                VARIABLES.map(|v| format!("{{{{{v}}}}}")).join(", ")
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
    exiger_des_pieces_pretes(state, id).await?;

    let mut tx = state.db().write(ctx).await?;
    let Some(partiront) = mailings::figer(&mut tx, id, acteur).await? else {
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
    tx.commit().await?;
    detail(state, acteur, id).await
}

/// Arrête un envoi en cours : ce qui est parti reste parti, le reste ne part
/// pas.
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
            "Seul un envoi en cours peut être arrêté.",
            "status",
        ));
    }
    tx.commit().await?;
    detail(state, acteur, id).await
}
