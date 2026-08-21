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
use crate::repo::{public_schedule, session_parts};

/// La page publique d'une séance : elle, ses intervenants, ses organisations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SeancePublique {
    pub session: PublicScheduleRow,
    pub speakers: Vec<serde_json::Value>,
    pub organizations: Vec<serde_json::Value>,
}

/// La programmation d'une édition.
pub async fn programmation(pool: &PgPool, event_id: EventId) -> Result<Vec<PublicScheduleRow>> {
    public_schedule::programmation(pool, event_id).await
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

    Ok(SeancePublique {
        speakers: session_parts::intervenants(pool, id).await?,
        organizations: session_parts::organisations(pool, id).await?,
        session,
    })
}
