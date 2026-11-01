//! Réunir et séparer : les séances communes du planificateur (01/11).
//!
//! Faute de créneaux pour tous, l'équipe réunit deux dossiers retenus ou plus
//! dans un créneau, sous un titre commun. Le public voit une séance : une
//! inscription, une réunion visio, un direct.
//!
//! # Rien n'est refusé pour chevauchement, ici non plus
//!
//! Seulement ce qui perdrait des inscrits (une séance retirée emporte ses
//! inscriptions par cascade) et ce qui n'est pas un dossier réunissable.
//!
//! # Les deux gestes rendent tout l'écran
//!
//! Réunir crée une séance et en retire plusieurs ; séparer fait l'inverse. Un
//! `PlannerMutationResult` ne décrit qu'une séance : l'écran recevrait une
//! grille fausse. Il reçoit donc `PlannerScreen`, composé dans la transaction.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use serde::Deserialize;
use sqlx::postgres::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{EventId, SessionId};
use crate::domain::sessions::PlannerScreen;
use crate::domain::slug;
use crate::repo::{joint, session_parts, sessions, themes};
use crate::state::ProgrammeState;

/// `JointSessionPayload` : les dossiers dans l'ordre d'affichage, le titre
/// commun.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct JointSessionPayload {
    pub event_id: Uuid,
    pub proposal_ids: Vec<Uuid>,
    /// `I18nText` : le français est exigé.
    pub title: serde_json::Value,
}

fn invalide(message: &str, champ: &str) -> ApiError {
    ApiError::with_message(ErrorCode::JointSessionInvalid, message).field(champ)
}

fn avec_inscrits() -> ApiError {
    ApiError::new(ErrorCode::JointSessionHasRegistrations)
}

/// Le titre commun, nettoyé : `{fr, en?}`, le français non vide.
fn titre_commun(titre: &serde_json::Value) -> Result<serde_json::Value> {
    let texte = |cle: &str| {
        titre
            .get(cle)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
    };
    let fr = texte("fr").ok_or_else(|| {
        ApiError::validation(
            "Le titre commun est obligatoire, en français au moins.",
            "title",
        )
    })?;
    let mut propre = serde_json::json!({ "fr": fr });
    if let Some(en) = texte("en") {
        propre["en"] = serde_json::Value::String(en);
    }
    Ok(propre)
}

/// Réunir des dossiers retenus d'une édition dans une séance commune, au
/// créneau de la séance du premier.
pub async fn reunir(
    state: &ProgrammeState,
    ctx: &RequestContext,
    event_id: EventId,
    payload: JointSessionPayload,
) -> Result<PlannerScreen> {
    let titre = titre_commun(&payload.title)?;

    let mut ids: Vec<Uuid> = Vec::with_capacity(payload.proposal_ids.len());
    for id in payload.proposal_ids {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() < 2 {
        return Err(invalide(
            "Choisissez au moins deux dossiers à réunir.",
            "proposal_ids",
        ));
    }

    let mut tx = state.db().write(ctx).await?;

    let dossiers = joint::dossiers(&mut tx, &ids).await?;
    // Un dossier d'une autre édition est traité comme absent : l'URL forgée
    // n'apprend pas qu'il existe.
    if dossiers.len() != ids.len() || dossiers.iter().any(|d| d.event_id != event_id.as_uuid()) {
        return Err(invalide(
            "Un de ces dossiers est introuvable dans cette édition.",
            "proposal_ids",
        ));
    }
    for d in &dossiers {
        if d.status != "accepted" {
            return Err(invalide(
                "Seuls des dossiers retenus peuvent être réunis.",
                "proposal_ids",
            ));
        }
        if d.deja_reuni {
            return Err(invalide(
                "Un de ces dossiers est déjà réuni dans une séance commune : séparez-la d'abord.",
                "proposal_ids",
            ));
        }
        if d.seances_closes > 0 {
            return Err(invalide(
                "Un de ces dossiers a une séance en cours ou terminée : elle ne peut plus être réunie.",
                "proposal_ids",
            ));
        }
        if d.inscrits > 0 {
            return Err(avec_inscrits().field("proposal_ids"));
        }
    }

    let Some(modele) = joint::seance_modele(&mut tx, ids[0]).await? else {
        return Err(invalide(
            "Le premier dossier n'a pas de séance dont reprendre le créneau.",
            "proposal_ids",
        ));
    };

    let base = slug_du_titre(&mut tx, &titre).await?;
    let session_id = creer(&mut tx, event_id, modele, &titre, &base, ctx.actor_id).await?;

    for (rang, &dossier) in ids.iter().enumerate() {
        let rang = i16::try_from(rang + 1).unwrap_or(i16::MAX);
        joint::attacher(&mut tx, session_id, dossier, rang, ctx.actor_id)
            .await
            .map_err(|e| traduire(&e))?;
        joint::reprendre_les_fils(&mut tx, session_id, dossier).await?;
        session_parts::recopier_les_intervenants(&mut tx, dossier, session_id).await?;
        session_parts::recopier_les_coorganisations(&mut tx, dossier, session_id).await?;
        themes::recopier_sur_la_seance(&mut tx, dossier, session_id).await?;
    }
    for &dossier in &ids {
        joint::retirer_les_seances(&mut tx, dossier).await?;
    }

    rendre(tx, event_id).await
}

/// Séparer une séance commune : chaque dossier retrouve une séance propre, le
/// premier au créneau de la séance commune, les autres au panneau « à placer ».
/// Les séances renaissantes ne sont pas publiées : l'équipe les place puis
/// republie.
pub async fn separer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    event_id: EventId,
    session_id: SessionId,
) -> Result<PlannerScreen> {
    let mut tx = state.db().write(ctx).await?;

    let Some(commune) = joint::seance_commune(&mut tx, session_id.as_uuid()).await? else {
        return Err(ApiError::not_found());
    };
    if commune.dossiers.is_empty() {
        return Err(invalide(
            "Cette séance n'est pas une séance commune.",
            "session_id",
        ));
    }
    if matches!(commune.status.as_str(), "live" | "completed") {
        return Err(invalide(
            "Cette séance est en cours ou terminée : elle ne peut plus être séparée.",
            "session_id",
        ));
    }
    if commune.inscrits > 0 {
        return Err(avec_inscrits().field("session_id"));
    }

    joint::supprimer(&mut tx, session_id.as_uuid()).await?;

    for &dossier in &commune.dossiers {
        crate::service::birth::faire_naitre(&mut tx, dossier).await?;
    }
    if let Some(premiere) = joint::seance_modele(&mut tx, commune.dossiers[0]).await? {
        sessions::ecrire_le_creneau(
            &mut tx,
            SessionId::from(premiere),
            sessions::Creneau {
                room_id: commune.room_id,
                starts_at: commune.starts_at,
                ends_at: commune.ends_at,
                event_day_id: commune.event_day_id,
            },
        )
        .await
        .map_err(|e| pg_error::translate(&e))?;
    }

    rendre(tx, event_id).await
}

/// La base d'adresse du titre commun, normalisée **en base** comme celle d'un
/// dossier (`platform.slugify()`).
async fn slug_du_titre(conn: &mut PgConnection, titre: &serde_json::Value) -> Result<String> {
    let fr = titre.get("fr").and_then(serde_json::Value::as_str);
    let brut = sqlx::query_scalar!(r#"SELECT platform.slugify($1)::text AS "s""#, fr)
        .fetch_one(conn)
        .await?;
    Ok(slug::base(brut.as_deref()))
}

/// Insérer la séance commune, en suffixant l'adresse sur collision (R7).
async fn creer(
    conn: &mut PgConnection,
    event_id: EventId,
    modele: Uuid,
    titre: &serde_json::Value,
    base: &str,
    acteur: Option<Uuid>,
) -> Result<Uuid> {
    for tentative in 0..slug::TENTATIVES_MAX {
        let adresse = slug::tentative(base, tentative);
        if sessions::adresse_prise(conn, event_id, &adresse).await? {
            continue;
        }
        return joint::creer(conn, modele, titre, &adresse, acteur)
            .await
            .map_err(|e| traduire(&e))?
            .ok_or_else(|| ApiError::internal("la séance modèle a disparu dans sa transaction"));
    }
    Err(ApiError::internal(
        "dix adresses d'URL déjà prises pour la même séance commune",
    ))
}

/// Le refus de `tg_check_session_proposal()` (23000) et l'unicité d'un dossier
/// réuni (course entre deux réunions) sont les seuls attendus.
fn traduire(erreur: &sqlx::Error) -> ApiError {
    match pg_error::sqlstate(erreur).as_deref() {
        Some("23000") => invalide(
            "Un de ces dossiers relève d'une autre édition.",
            "proposal_ids",
        ),
        Some("23505") if pg_error::constraint(erreur) == Some("ux_session_proposals_proposal") => {
            invalide(
                "Un de ces dossiers est déjà réuni dans une séance commune : séparez-la d'abord.",
                "proposal_ids",
            )
        }
        _ => pg_error::translate(erreur),
    }
}

async fn rendre(mut tx: kernel::db::WriteTx, event_id: EventId) -> Result<PlannerScreen> {
    let ecran = crate::service::planner::composer(&mut tx, event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;
    Ok(ecran)
}
