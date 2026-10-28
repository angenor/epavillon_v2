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
//! cadence réglée (`engagement.mailing.hourly_limit`).

use kernel::auth::{administered_events, has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::jobs::{self, NewJob};
use kernel::RequestContext;
use uuid::Uuid;

use crate::domain::mailing::{
    normaliser_adresses, CreateMailingPayload, Mailing, MailingAudience, MailingCriterion,
    MailingListScreen, MailingPayload, MailingTestResult, KINDS, VARIABLES,
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
    let (hourly_limit, team_cc) = mailings::reglages(state.pool()).await?;
    Ok(MailingListScreen {
        rows: mailings::lister(state.pool(), event_id).await?,
        hourly_limit,
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
    let (hourly_limit, _) = mailings::reglages(state.pool()).await?;
    Ok(Mailing {
        counts: mailings::compter(&recipients),
        recipients,
        hourly_limit,
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

/// La liste telle qu'elle partirait maintenant, avec le décompte et la durée
/// estimée à la cadence réglée.
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
    let (hourly_limit, _) = mailings::reglages(state.pool()).await?;

    let total = entries.len() as i64;
    let suppressed = entries.iter().filter(|e| e.is_suppressed).count() as i64;
    let sendable = total - suppressed;
    let without_first_name = entries
        .iter()
        .filter(|e| !e.is_suppressed && e.first_name.is_none())
        .count() as i64;
    let par_message = 1 + envoi.cc.len() as i64 + envoi.bcc.len() as i64;
    let envelope_count = sendable * par_message;
    let estimated_minutes = (envelope_count * 60 + hourly_limit - 1) / hourly_limit;

    Ok(MailingAudience {
        total,
        sendable,
        suppressed,
        without_first_name,
        envelope_count,
        hourly_limit,
        estimated_minutes,
        entries,
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
        }
    }
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

    let mut citees = render::variables_citees(&subject);
    citees.extend(render::variables_citees(&body_html));
    if let Some(inconnue) = citees.iter().find(|v| !VARIABLES.contains(&v.as_str())) {
        return Err(ApiError::validation(
            format!(
                "Variable inconnue : {{{{{inconnue}}}}}. Possibles : {}.",
                VARIABLES.map(|v| format!("{{{{{v}}}}}")).join(", ")
            ),
            "body_html",
        ));
    }

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

    Ok(Brouillon {
        subject,
        body_html,
        audience,
        extra_addresses,
        cc,
        bcc,
        reply_to,
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

    let mut cc_par_defaut = mailings::reglages(state.pool()).await?.1;
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

    let mut tx = state.db().write(ctx).await?;
    if !mailings::modifier(&mut tx, id, &brouillon.valeurs()).await? {
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
