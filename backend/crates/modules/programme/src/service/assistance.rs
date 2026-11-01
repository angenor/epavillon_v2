//! **L'assistance sur toute la plateforme** (02/11, demande du commanditaire).
//!
//! Une fenêtre sur chaque écran : côté organisation, les sujets de ses dossiers
//! et ses questions ou suggestions sur la plateforme ; côté équipe, tout ce qui
//! attend une réponse, dossiers du périmètre et, en portée globale, plateforme.
//!
//! Un sujet sur un dossier reste un échange du dossier : il s'ouvre et se
//! répond par les routes du dossier, qui gardent leurs règles (visibilité,
//! demande de correction). Ce service lit l'ensemble, ouvre et répond aux
//! sujets **sans dossier**, et tient la lecture des deux.
//!
//! # Qui voit la fenêtre côté organisation
//!
//! Qui suit au moins un dossier dans `ETATS_QUI_OUVRENT_L_ASSISTANCE`. Le
//! commanditaire pourra la réserver plus tard aux dossiers retenus (contre les
//! sujets ouverts par des dépôts de complaisance) : c'est cette liste qui change.

use kernel::auth::{self, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::permissions::SUPPORT_ANSWER;
use crate::repo::comments::{self, Message, NouveauMessage};
use crate::repo::cross::assistance::{self as lecture, LigneDAssistance, Place};
use crate::service::comments::VISIBILITE_DEPOSANT;
use crate::state::ProgrammeState;

/// Les états d'un dossier qui ouvrent l'assistance à son organisation.
pub const ETATS_QUI_OUVRENT_L_ASSISTANCE: &[&str] =
    &["submitted", "under_review", "changes_requested", "accepted"];

const COTE_EQUIPE: &str = "team";
const COTE_ORGANISATION: &str = "organization";
const SUJET_MAX: usize = 120;

/// `AssistanceStatus` : de quel côté la personne se tient (nul : pas
/// d'assistance pour elle), et ce qu'elle n'a pas lu.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssistanceStatus {
    pub side: Option<&'static str>,
    pub unread: i64,
}

/// `AssistanceMessage`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssistanceMessage {
    pub id: Uuid,
    /// Nul : sujet sur la plateforme.
    pub proposal_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    /// Nul côté organisation quand l'équipe écrit : elle signe « l'équipe ».
    pub author_name: Option<String>,
    pub from_team: bool,
    pub body: String,
    pub is_change_request: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub edited_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub topic: Option<String>,
    pub topic_other: Option<String>,
    pub subject: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub closed_at: Option<OffsetDateTime>,
    pub closed_by: Option<Uuid>,
    pub is_read: bool,
    pub reference_code: Option<String>,
    pub proposal_title: Option<Value>,
    /// Sur le message qui ouvre un sujet seulement.
    pub organization_name: Option<String>,
}

/// `AssistanceProposal` : un dossier sur lequel ouvrir un sujet.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssistanceProposal {
    pub id: Uuid,
    pub reference_code: String,
    pub title: Value,
}

/// `AssistanceFeed`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssistanceFeed {
    pub side: &'static str,
    pub messages: Vec<AssistanceMessage>,
    /// Côté organisation : ses dossiers ouverts à l'assistance. Vide côté équipe.
    pub proposals: Vec<AssistanceProposal>,
    /// Côté organisation : les organisations à suivre en temps réel.
    pub organization_ids: Vec<Uuid>,
}

/// `OpenSupportSubjectPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OpenSupportSubjectPayload {
    /// Code de `platform_support_topic`.
    pub topic: String,
    pub subject: String,
    pub body: String,
}

/// `ReplySupportSubjectPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReplySupportSubjectPayload {
    pub body: String,
}

/// `MarkAssistanceReadPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MarkAssistanceReadPayload {
    pub comment_ids: Vec<Uuid>,
}

async fn place_de(state: &ProgrammeState, personne: Uuid) -> Result<Option<Place>> {
    let pool = state.pool();
    if auth::has_permission_anywhere(pool, personne, SUPPORT_ANSWER).await? {
        let globale = auth::has_permission(pool, personne, SUPPORT_ANSWER, Scope::Global).await?;
        let editions = auth::administered_events(pool, personne).await?.event_ids;
        return Ok(Some(Place::Equipe { globale, editions }));
    }
    let suivis = lecture::dossiers_suivis(pool, personne, ETATS_QUI_OUVRENT_L_ASSISTANCE).await?;
    Ok((!suivis.is_empty()).then_some(Place::Organisation))
}

async fn place_exigee(state: &ProgrammeState, personne: Uuid) -> Result<Place> {
    place_de(state, personne)
        .await?
        .ok_or_else(ApiError::forbidden)
}

fn cote(place: &Place) -> &'static str {
    match place {
        Place::Equipe { .. } => COTE_EQUIPE,
        Place::Organisation => COTE_ORGANISATION,
    }
}

/// `GET /assistance`.
pub async fn etat(state: &ProgrammeState, personne: Uuid) -> Result<AssistanceStatus> {
    let Some(place) = place_de(state, personne).await? else {
        return Ok(AssistanceStatus {
            side: None,
            unread: 0,
        });
    };
    Ok(AssistanceStatus {
        side: Some(cote(&place)),
        unread: lecture::non_lus(state.pool(), personne, &place).await?,
    })
}

fn vue(ligne: LigneDAssistance, place: &Place) -> AssistanceMessage {
    let signe_l_equipe = *place == Place::Organisation && ligne.from_team;
    AssistanceMessage {
        id: ligne.id,
        proposal_id: ligne.proposal_id,
        parent_id: ligne.parent_id,
        author_id: ligne.author_id,
        author_name: (!signe_l_equipe).then_some(ligne.author_name),
        from_team: ligne.from_team,
        body: ligne.body,
        is_change_request: ligne.is_change_request,
        resolved_at: ligne.resolved_at,
        edited_at: ligne.edited_at,
        created_at: ligne.created_at,
        topic: ligne.topic,
        topic_other: ligne.topic_other,
        subject: ligne.subject,
        closed_at: ligne.closed_at,
        closed_by: ligne.closed_by,
        is_read: ligne.is_read,
        reference_code: ligne.reference_code,
        proposal_title: ligne.proposal_title,
        organization_name: ligne.organization_name,
    }
}

/// `GET /assistance/subjects`.
pub async fn fil(state: &ProgrammeState, personne: Uuid) -> Result<AssistanceFeed> {
    let place = place_exigee(state, personne).await?;
    let messages = lecture::messages(state.pool(), personne, &place)
        .await?
        .into_iter()
        .map(|l| vue(l, &place))
        .collect();

    let (proposals, organization_ids) = if place == Place::Organisation {
        let suivis =
            lecture::dossiers_suivis(state.pool(), personne, ETATS_QUI_OUVRENT_L_ASSISTANCE)
                .await?;
        let mut organisations: Vec<Uuid> = suivis.iter().map(|d| d.organization_id).collect();
        organisations.sort_unstable();
        organisations.dedup();
        let dossiers = suivis
            .into_iter()
            .map(|d| AssistanceProposal {
                id: d.id,
                reference_code: d.reference_code,
                title: d.title,
            })
            .collect();
        (dossiers, organisations)
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(AssistanceFeed {
        side: cote(&place),
        messages,
        proposals,
        organization_ids,
    })
}

fn corps_exige(corps: &str) -> Result<&str> {
    let corps = corps.trim();
    if corps.is_empty() {
        return Err(ApiError::validation(
            "Un message ne peut pas être vide.",
            "body",
        ));
    }
    Ok(corps)
}

/// `POST /assistance/subjects` : une question ou une suggestion sur la
/// plateforme. Réservé à l'organisation : l'équipe répond, elle n'ouvre pas.
pub async fn ouvrir(
    state: &ProgrammeState,
    ctx: &RequestContext,
    auteur: Uuid,
    payload: OpenSupportSubjectPayload,
) -> Result<Message> {
    if place_exigee(state, auteur).await? != Place::Organisation {
        return Err(ApiError::forbidden());
    }
    let corps = corps_exige(&payload.body)?;
    let titre = payload.subject.trim();
    if !(2..=SUJET_MAX).contains(&titre.chars().count()) {
        return Err(ApiError::validation(
            "Le titre du sujet compte de 2 à 120 caractères.",
            "subject",
        ));
    }
    let nature = comments::terme_de_nature(
        state.pool(),
        comments::NATURES_D_ASSISTANCE,
        payload.topic.trim(),
    )
    .await?
    .ok_or_else(|| ApiError::validation("Choisissez la nature de la demande.", "topic"))?;

    let mut tx = state.db().write(ctx).await?;
    let message = comments::ecrire(
        &mut tx,
        None,
        auteur,
        &NouveauMessage {
            parent_id: None,
            visibility: VISIBILITE_DEPOSANT,
            body: corps,
            is_change_request: false,
            topic_term_id: Some(nature),
            topic_other: None,
            subject: Some(titre),
        },
    )
    .await?;
    crate::realtime::signaler_assistance(&mut tx, auteur).await?;
    tx.commit().await?;
    Ok(message)
}

/// La racine d'un sujet sans dossier que cette personne lit.
async fn sujet_sans_dossier(
    state: &ProgrammeState,
    personne: Uuid,
    place: &Place,
    racine: &Message,
) -> Result<()> {
    if racine.parent_id.is_some() || racine.proposal_id.is_some() {
        return Err(ApiError::not_found());
    }
    if !lecture::lisible(state.pool(), personne, place, racine.id).await? {
        return Err(ApiError::not_found());
    }
    Ok(())
}

/// `POST /assistance/subjects/{id}/replies`. Une réponse rouvre un sujet résolu
/// (base).
pub async fn repondre(
    state: &ProgrammeState,
    ctx: &RequestContext,
    auteur: Uuid,
    racine: Uuid,
    payload: ReplySupportSubjectPayload,
) -> Result<Message> {
    let place = place_exigee(state, auteur).await?;
    let racine = comments::par_id(state.pool(), crate::domain::ids::CommentId(racine))
        .await?
        .ok_or_else(ApiError::not_found)?;
    sujet_sans_dossier(state, auteur, &place, &racine).await?;
    let corps = corps_exige(&payload.body)?;

    let mut tx = state.db().write(ctx).await?;
    let message = comments::ecrire(
        &mut tx,
        None,
        auteur,
        &NouveauMessage {
            parent_id: Some(racine.id),
            visibility: VISIBILITE_DEPOSANT,
            body: corps,
            is_change_request: false,
            topic_term_id: None,
            topic_other: None,
            subject: None,
        },
    )
    .await?;
    crate::realtime::signaler_assistance(&mut tx, racine.author_id).await?;
    tx.commit().await?;
    Ok(message)
}

/// Résoudre ou rouvrir un sujet sans dossier ; appelé par la route commune
/// des sujets (`/proposal-comments/{id}/closure`).
pub(crate) async fn clore(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    racine: &Message,
    clos: bool,
) -> Result<Message> {
    let place = place_exigee(state, acteur).await?;
    sujet_sans_dossier(state, acteur, &place, racine).await?;

    let mut tx = state.db().write(ctx).await?;
    let ligne = comments::clore(&mut tx, racine.id, clos, acteur).await?;
    crate::realtime::signaler_assistance(&mut tx, racine.author_id).await?;
    tx.commit().await?;
    Ok(ligne)
}

/// `POST /assistance/read` : marquer lus, sujets de dossier comme sans
/// dossier. Ce que la personne ne lit pas est écarté par la requête.
pub async fn marquer_lus(
    state: &ProgrammeState,
    ctx: &RequestContext,
    lecteur: Uuid,
    payload: MarkAssistanceReadPayload,
) -> Result<()> {
    if payload.comment_ids.is_empty() {
        return Ok(());
    }
    let place = place_exigee(state, lecteur).await?;
    let mut tx = state.db().write(ctx).await?;
    lecture::marquer_lus(&mut tx, lecteur, &place, &payload.comment_ids).await?;
    tx.commit().await?;
    Ok(())
}

/// Remettre en non lu un message d'un sujet sans dossier ; appelé par la
/// route commune (`DELETE /proposal-comments/{id}/read`).
pub(crate) async fn marquer_non_lu(
    state: &ProgrammeState,
    ctx: &RequestContext,
    lecteur: Uuid,
    message: &Message,
) -> Result<()> {
    let place = place_exigee(state, lecteur).await?;
    if !lecture::lisible(state.pool(), lecteur, &place, message.id).await? {
        return Err(ApiError::not_found());
    }
    let mut tx = state.db().write(ctx).await?;
    comments::marquer_non_lu(&mut tx, message.id, lecteur).await?;
    tx.commit().await?;
    Ok(())
}
