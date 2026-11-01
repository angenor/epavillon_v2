//! La programmation publique — **aucune session exigée**.
//!
//! # Ce que ce service ne fait presque pas
//!
//! Il lit la vue et rend ses lignes. Le filtre sur les séances publiées, le
//! repli de couverture, l'état temporel et les décomptes vivent en base, dans
//! `v_public_schedule` : les rejouer ici produirait une seconde vérité que
//! personne ne saurait départager.
//!
//! # Vide, jamais une erreur
//!
//! Une édition dont le programme n'est pas paru rend une liste vide. C'est un
//! état normal — l'écran l'annonce et n'invente rien —, et un refus obligerait
//! le site à distinguer « pas encore publié » de « édition inexistante », ce que
//! précisément il ne doit pas faire.

use kernel::error::{ApiError, Result};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::domain::ids::{EventId, SessionId};
use crate::domain::sessions::PublicScheduleRow;
use crate::repo::{joint, public_schedule, session_parts};

/// La page publique d'une séance : elle, ses intervenants, ses organisations.
/// Les deux listes sont les lectures **publiques** de `session_parts`, jamais
/// celles du back-office.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SeancePublique {
    pub session: PublicScheduleRow,
    pub description: Option<serde_json::Value>,
    pub allows_questions: bool,
    pub is_recorded: bool,
    /// Motif public d'une séance annulée ; nul sinon.
    pub cancelled_reason: Option<serde_json::Value>,
    pub speakers: Vec<serde_json::Value>,
    pub organizations: Vec<serde_json::Value>,
    /// Les photos de la galerie (D7), prêtes et publiques, dans l'ordre réglé.
    pub gallery: Vec<serde_json::Value>,
    /// Séance commune : un élément par dossier réuni (`JointProposalDetail`),
    /// dans l'ordre d'affichage ; vide pour une séance ordinaire.
    pub proposals: Vec<serde_json::Value>,
}

/// La programmation d'une édition — ou, **sans édition**, les séances à venir
/// de toutes les éditions, plafonnées.
pub async fn programmation(
    pool: &PgPool,
    event_id: Option<EventId>,
    limite: Option<i64>,
) -> Result<Vec<PublicScheduleRow>> {
    public_schedule::programmation(pool, event_id, limite).await
}

/// Le détail d'une séance **publiée**, par son adresse d'URL.
///
/// **Une adresse inconnue et une séance non publiée rendent le même refus** :
/// la vue ne porte que le publié, et distinguer les deux dirait au public
/// qu'une séance existe sans être encore annoncée.
pub async fn seance(pool: &PgPool, event_id: EventId, slug: &str) -> Result<SeancePublique> {
    let session = public_schedule::par_adresse(pool, event_id, slug)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let id = SessionId::from(session.id);
    let complement = public_schedule::complement(pool, id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let speakers = session_parts::intervenants_publics(pool, id).await?;
    let organizations = session_parts::organisations_publiques(pool, id).await?;
    let proposals = parties(pool, &session, &speakers, &organizations).await?;

    Ok(SeancePublique {
        description: complement.description,
        allows_questions: complement.allows_questions,
        is_recorded: complement.is_recorded,
        cancelled_reason: complement.cancelled_reason,
        speakers,
        organizations,
        gallery: session_parts::galerie_publique(pool, id).await?,
        proposals,
        session,
    })
}

/// Les dossiers d'une séance commune, chacun avec ce que la vue en porte déjà
/// (titre d'origine, couverture, porteur), sa présentation, ses intervenants
/// et ses co-organisations, triés dans les listes plates par leur dossier.
async fn parties(
    pool: &PgPool,
    session: &PublicScheduleRow,
    intervenants: &[serde_json::Value],
    organisations: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>> {
    let serde_json::Value::Array(dossiers) = &session.proposals else {
        return Ok(Vec::new());
    };
    if dossiers.is_empty() {
        return Ok(Vec::new());
    }

    let descriptions = joint::parties(pool, session.id).await?;
    let du_dossier = |lignes: &[serde_json::Value], dossier: &str, sans_porteur: bool| {
        lignes
            .iter()
            .filter(|l| l.get("proposal_id").and_then(serde_json::Value::as_str) == Some(dossier))
            .filter(|l| {
                !sans_porteur || l.get("role").and_then(serde_json::Value::as_str) != Some("lead")
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    Ok(dossiers
        .iter()
        .map(|d| {
            let mut partie = d.clone();
            let dossier = d
                .get("proposal_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let description = descriptions
                .iter()
                .find(|(id, _)| id.to_string() == dossier)
                .and_then(|(_, texte)| texte.clone())
                .unwrap_or(serde_json::Value::Null);
            partie["description"] = description;
            partie["speakers"] =
                serde_json::Value::Array(du_dossier(intervenants, &dossier, false));
            partie["organizations"] =
                serde_json::Value::Array(du_dossier(organisations, &dossier, true));
            partie
        })
        .collect())
}
