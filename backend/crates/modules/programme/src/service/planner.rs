//! L'écran d'arbitrage et ses trois écritures.
//!
//! # 🔴 AUCUNE de ces écritures ne peut être refusée pour chevauchement
//!
//! C'est le contrat le plus important du module, et il tient dans les types :
//! `PlannerMutationResult` ne porte **aucun discriminant de refus**. Le modèle ne
//! pose aucune contrainte d'exclusion sur les créneaux (décision structurante
//! n° 1 du fichier `075`), l'API n'en ajoute pas, et un écran qui ferait
//! autrement transformerait l'outil d'arbitrage en mur. L'équipe travaille par
//! déplacements successifs, en passant par des états incohérents — deux blocs
//! superposés le temps de recaler le second.
//!
//! Ce qui est refusé ici l'est pour une autre raison : une valeur **déduite**
//! qu'on essaie de saisir, une salle qui n'existe pas dans l'édition, une fin
//! antérieure au début.
//!
//! # L'écran se lit en UNE transaction, sur UNE connexion
//!
//! Les sept lectures sont exécutées dans une transaction `READ ONLY`. Les
//! conflits sont calculés **sur les séances** : lus à un autre instant, ils
//! décriraient une grille que l'écran n'affiche pas — le bandeau annoncerait un
//! chevauchement entre deux blocs dont l'un vient d'être déplacé (R10). Et une
//! transaction qui retiendrait deux connexions du pool sortirait en « service
//! indisponible » sous charge : c'est la leçon de B2.
//!
//! # Les écritures rendent les conflits LUS DANS LA TRANSACTION
//!
//! `detect_conflicts()` est `STABLE` et lit `programme.sessions` : appelée dans
//! la transaction, elle voit l'écriture non encore validée. Appelée après
//! validation, elle rendrait l'état d'une édition qu'une écriture concurrente a
//! pu changer entre-temps, et l'écran afficherait une grille et des conflits qui
//! ne se correspondent pas (R11).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use serde::Deserialize;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::derived;
use crate::domain::ids::{EventId, SessionId};
use crate::domain::sessions::{PlannerMutationResult, PlannerScreen};
use crate::repo::{conflicts, cross, planner, session_parts, sessions};
use crate::state::ProgrammeState;

// -----------------------------------------------------------------------------
// L'écran
// -----------------------------------------------------------------------------

/// Tout l'écran en une réponse. `None` quand l'édition n'existe pas.
pub async fn ecran(pool: &PgPool, event_id: EventId) -> Result<Option<PlannerScreen>> {
    let mut tx = pool.begin().await?;

    // `SET TRANSACTION` doit précéder toute lecture : posée après la première
    // requête, PostgreSQL la refuse.
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;

    let ecran = composer(&mut tx, event_id).await?;

    // Une transaction en lecture seule n'a rien à valider.
    tx.rollback().await?;

    Ok(ecran)
}

async fn composer(conn: &mut PgConnection, event_id: EventId) -> Result<Option<PlannerScreen>> {
    let Some(edition) = cross::contexte_edition(&mut *conn, event_id).await? else {
        return Ok(None);
    };
    let Some(fiche) = cross::fiche_edition(&mut *conn, event_id).await? else {
        return Ok(None);
    };

    let seances = planner::seances_de_ledition(&mut *conn, event_id).await?;
    // **Une séance sans salle est au panneau, jamais dans la grille**, et
    // réciproquement : c'est la seule chose qui les distingue.
    let (placed, unplaced) = seances.into_iter().partition(|s| s.room_id.is_some());

    Ok(Some(PlannerScreen {
        event_id: event_id.as_uuid(),
        event_title: fiche.title,
        timezone: edition.timezone,
        zone_label: edition.city,
        programme_published_at: edition.programme_published_at,
        days: cross::jours_de_ledition(&mut *conn, event_id).await?,
        rooms: cross::salles_de_ledition(&mut *conn, event_id).await?,
        tracks: cross::fils_de_ledition(&mut *conn, event_id).await?,
        channels: cross::canaux_applicables(&mut *conn, event_id).await?,
        placed,
        unplaced,
        conflicts: conflicts::conflits(&mut *conn, event_id).await?,
    }))
}

/// Les séances d'une édition, seules — `GET /sessions`.
pub async fn seances(
    pool: &PgPool,
    event_id: EventId,
) -> Result<Vec<crate::domain::sessions::PlannerSession>> {
    planner::seances_de_ledition(pool, event_id).await
}

/// Les conflits d'une édition, seuls — `GET /sessions/conflicts`.
pub async fn conflits(
    pool: &PgPool,
    event_id: EventId,
) -> Result<Vec<crate::domain::sessions::ScheduleConflict>> {
    conflicts::conflits(pool, event_id).await
}

// -----------------------------------------------------------------------------
// Les trois écritures
// -----------------------------------------------------------------------------

/// Placer, déplacer, redimensionner, retirer — `ScheduleSessionPayload`.
///
/// **Une seule écriture pour les quatre gestes** : la base n'en distingue pas,
/// ce sont les colonnes `room_id`, `starts_at` et `ends_at`. Quatre routes
/// auraient donné quatre occasions de diverger sur la détection des conflits,
/// qui est justement ce que l'écran doit rendre identique dans les quatre.
///
/// `room_id` nul **renvoie la séance au panneau** ; ce n'est pas une suppression.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScheduleSessionPayload {
    /// Envoyé par le front, **ignoré** : l'identifiant qui fait foi est celui de
    /// l'adresse.
    #[serde(default)]
    pub session_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    /// Journée de rattachement, **facultative**. Non fournie, elle est remise à
    /// nul pour que la base la redéduise (R9, écart n° 113).
    #[serde(default)]
    pub event_day_id: Option<Uuid>,
    /// Deux valeurs **déduites** que le contrat ne porte pas et qu'un client
    /// pourrait envoyer : elles sont refusées en nommant leur champ.
    #[serde(default)]
    pub time_range: Option<serde_json::Value>,
    #[serde(default)]
    pub enforce_room_exclusivity: Option<bool>,
}

/// Écrire le créneau d'une séance.
pub async fn placer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    event_id: EventId,
    session_id: SessionId,
    payload: ScheduleSessionPayload,
) -> Result<PlannerMutationResult> {
    if payload.time_range.is_some() {
        return Err(derived::refuser_lintervalle());
    }
    if payload.enforce_room_exclusivity.is_some() {
        return Err(derived::refuser_lexclusivite());
    }
    if payload.ends_at <= payload.starts_at {
        return Err(derived::refuser_le_creneau());
    }

    let mut tx = state.db().write(ctx).await?;

    // **La salle doit appartenir à l'édition de la séance.** Ni la base ni aucun
    // déclencheur ne le vérifient — seul le fil de programmation est contrôlé —,
    // et sans ce contrôle une URL forgée installerait une séance de la COP31
    // dans une salle de la COP30.
    if let Some(room_id) = payload.room_id {
        if !cross::salle_de_ledition(&mut *tx, event_id, room_id).await? {
            return Err(derived::reference_inconnue("room_id", "Cette salle"));
        }
    }
    if let Some(day_id) = payload.event_day_id {
        if !cross::jour_de_ledition(&mut *tx, event_id, day_id).await? {
            return Err(derived::reference_inconnue("event_day_id", "Cette journée"));
        }
    }

    sessions::ecrire_le_creneau(
        &mut tx,
        session_id,
        sessions::Creneau {
            room_id: payload.room_id,
            starts_at: payload.starts_at,
            ends_at: payload.ends_at,
            event_day_id: payload.event_day_id,
        },
    )
    .await
    .map_err(|e| traduire_le_creneau(&e))?;

    rendre(tx, event_id, session_id).await
}

/// La liste des journées spéciales — `SessionTracksPayload`.
///
/// **Manuel et indépendant de la date** : toutes les activités du 12 novembre ne
/// relèvent pas de la « Journée finance durable » (règle métier n° 7). La liste
/// envoyée **remplace** la précédente.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SessionTracksPayload {
    #[serde(default)]
    pub session_id: Option<Uuid>,
    pub track_ids: Vec<Uuid>,
}

pub async fn rattacher_les_fils(
    state: &ProgrammeState,
    ctx: &RequestContext,
    event_id: EventId,
    session_id: SessionId,
    payload: SessionTracksPayload,
) -> Result<PlannerMutationResult> {
    let mut tx = state.db().write(ctx).await?;

    session_parts::remplacer_les_fils(&mut tx, session_id, &payload.track_ids, ctx.actor_id)
        .await
        .map_err(|e| traduire_le_fil(&e))?;

    rendre(tx, event_id, session_id).await
}

/// La diffusion et son canal — `SessionBroadcastPayload`.
///
/// **Le canal EST saisissable** quand la diffusion est activée : le déclencheur
/// ne pose le canal par défaut que lorsque la colonne est nulle, il complète et
/// n'écrase jamais. L'écran laisse le choix quand l'édition a plusieurs canaux
/// (R8, écart n° 111).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SessionBroadcastPayload {
    #[serde(default)]
    pub session_id: Option<Uuid>,
    pub is_streamed: bool,
    #[serde(default)]
    pub broadcast_channel_id: Option<Uuid>,
}

pub async fn diffuser(
    state: &ProgrammeState,
    ctx: &RequestContext,
    event_id: EventId,
    session_id: SessionId,
    payload: SessionBroadcastPayload,
) -> Result<PlannerMutationResult> {
    // Retirer la diffusion **en désignant un canal** est refusé : c'est le seul
    // cas où la base efface une valeur choisie sans le dire.
    let canal = derived::canal_a_lecriture(payload.is_streamed, payload.broadcast_channel_id)?;

    let mut tx = state.db().write(ctx).await?;

    if let Some(channel_id) = canal {
        if !cross::canal_applicable(&mut *tx, event_id, channel_id).await? {
            return Err(derived::reference_inconnue(
                "broadcast_channel_id",
                "Ce canal de diffusion",
            ));
        }
    }

    sessions::ecrire_la_diffusion(&mut tx, session_id, payload.is_streamed, canal)
        .await
        .map_err(|e| pg_error::translate(&e))?;

    rendre(tx, event_id, session_id).await
}

/// Composer la réponse **dans la transaction, après l'écriture** : la séance et
/// les conflits de **toute l'édition** (R11).
async fn rendre(
    mut tx: kernel::db::WriteTx,
    event_id: EventId,
    session_id: SessionId,
) -> Result<PlannerMutationResult> {
    let session = planner::seance_du_planificateur(&mut *tx, event_id, session_id.as_uuid())
        .await?
        .ok_or_else(ApiError::not_found)?;
    let conflicts = conflicts::conflits(&mut *tx, event_id).await?;

    tx.commit().await?;

    Ok(PlannerMutationResult { session, conflicts })
}

/// **La traduction se fait par le SQLSTATE, jamais par le texte** : brancher sur
/// le message français d'une contrainte produirait un second libellé qui se
/// périmerait à la première évolution du SQL.
///
/// `ck_sessions_period` est rendue **sur le champ de fin** — celui que l'écran
/// vient de bouger en redimensionnant un bloc. Une clé étrangère qui casse
/// désigne une référence inconnue ; le service a déjà vérifié salle et journée,
/// donc ce cas ne survient qu'en course avec une suppression.
fn traduire_le_creneau(erreur: &sqlx::Error) -> ApiError {
    match pg_error::sqlstate(erreur).as_deref() {
        Some("23514") if pg_error::constraint(erreur) == Some("ck_sessions_period") => {
            derived::refuser_le_creneau()
        }
        Some("23503") => derived::reference_inconnue("room_id", "Cette salle"),
        _ => pg_error::translate(erreur),
    }
}

/// Le refus de `tg_check_session_track_event()`.
///
/// Le déclencheur lève `integrity_constraint_violation` — **relevé sur la base**,
/// il se traduit en 23000. Une clé étrangère qui casse désigne un fil qui
/// n'existe pas du tout, et le message le distingue.
fn traduire_le_fil(erreur: &sqlx::Error) -> ApiError {
    match pg_error::sqlstate(erreur).as_deref() {
        Some("23000") => ApiError::new(ErrorCode::SessionTrackEventMismatch).field("track_ids"),
        Some("23503") => derived::reference_inconnue("track_ids", "Cette journée spéciale"),
        _ => pg_error::translate(erreur),
    }
}
