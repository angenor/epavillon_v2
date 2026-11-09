//! Les intervenants en ligne (08/11) : la place Zoom de chacun dans la réunion
//! qui diffuse son activité (la sienne, ou celle de sa journée), et le courriel
//! de son lien personnel, qui ouvre l'application Zoom.
//!
//! Une adresse déjà inscrite comme public garde sa ligne : son lien est le
//! même, il suffit de le lui envoyer.

use kernel::auth::Perimeter;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Serialize;
use sqlx::postgres::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::repo::cross::intervenants::{self as lecture, IntervenantJoignable};
use crate::repo::cross::programme;
use crate::repo::intervenants::{self as repo, NouvelIntervenant};
use crate::repo::meetings::{self, NouveauParticipant};
use crate::service::{inscrits, meeting};
use crate::state::LiveState;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SpeakerZoomAccess {
    pub join_url: String,
}

/// La réunion qui diffuse la séance, créée s'il le faut ; aucune pour une
/// séance qui ne se suit pas en ligne.
pub async fn reunion_de(conn: &mut PgConnection, session_id: Uuid) -> Result<Option<Uuid>> {
    let Some(seance) = programme::seance(conn, session_id).await? else {
        return Ok(None);
    };
    if !seance.is_streamed || seance.status == "cancelled" {
        return Ok(None);
    }
    inscrits::assurer_reunion_du_direct(conn, &seance).await?;
    let reunion = match programme::journee_de_seance(conn, session_id).await? {
        Some(track_id) => meetings::reunion_active_journee(conn, track_id).await?,
        None => meetings::reunion_active(conn, session_id).await?,
    };
    Ok(reunion.map(|r| r.id))
}

/// La place de l'intervenant dans la réunion : la sienne si elle vit, reprise
/// si elle avait été annulée, neuve sinon.
pub async fn assurer_place(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    i: &IntervenantJoignable,
) -> Result<repo::PlaceDIntervenant> {
    match repo::place(conn, meeting_id, &i.email).await? {
        Some(p) if !p.cancelled => return Ok(p),
        Some(p) => {
            meetings::reprendre_participant(
                conn,
                p.id,
                &NouveauParticipant {
                    meeting_id,
                    registration_id: None,
                    track_registration_id: None,
                    person_id: i.person_id,
                    display_name: &i.display_name,
                    email: &i.email,
                    first_name: &i.first_name,
                    last_name: &i.last_name,
                },
            )
            .await?;
        }
        None => {
            repo::inserer(
                conn,
                &NouvelIntervenant {
                    meeting_id,
                    person_id: i.person_id,
                    display_name: &i.display_name,
                    email: &i.email,
                    first_name: &i.first_name,
                    last_name: &i.last_name,
                },
            )
            .await?;
        }
    }
    repo::place(conn, meeting_id, &i.email)
        .await?
        .ok_or_else(|| ApiError::internal("place d'intervenant disparue"))
}

/// Places et courriels des intervenants d'une séance. `auto` (la veille) : qui
/// a déjà reçu son lien n'en reçoit pas un second. Rend le nombre de demandes.
pub async fn donner_les_places(
    conn: &mut PgConnection,
    session_id: Uuid,
    ids: Option<&[Uuid]>,
    auto: bool,
) -> Result<usize> {
    let intervenants = lecture::joignables(conn, session_id, ids).await?;
    if intervenants.is_empty() {
        return Ok(0);
    }
    let Some(reunion) = reunion_de(conn, session_id).await? else {
        return Ok(0);
    };
    let mut demandes = 0;
    for i in &intervenants {
        let place = assurer_place(conn, reunion, i).await?;
        if auto && place.link_requested_at.is_some() {
            continue;
        }
        repo::demander_le_lien(conn, place.id, session_id).await?;
        demandes += 1;
    }
    Ok(demandes)
}

/// Le lien Zoom d'un intervenant, à l'écran : la place se crée si elle manque,
/// et le lien se rend dès que Zoom l'a donné (409 avant : réessayer).
async fn lien(
    conn: &mut PgConnection,
    session_id: Uuid,
    speaker_id: Uuid,
) -> Result<SpeakerZoomAccess> {
    let not_ready = || ApiError::new(ErrorCode::LiveMeetingNotReady);
    let i = lecture::joignables(conn, session_id, Some(&[speaker_id]))
        .await?
        .into_iter()
        .next()
        .ok_or_else(ApiError::not_found)?;
    let reunion = reunion_de(conn, session_id).await?.ok_or_else(not_ready)?;
    let place = assurer_place(conn, reunion, &i).await?;
    place
        .personal_join_url
        .map(|join_url| SpeakerZoomAccess { join_url })
        .ok_or_else(not_ready)
}

pub async fn lien_par_le_jeton(
    state: &LiveState,
    jeton: &str,
    speaker_id: Uuid,
) -> Result<SpeakerZoomAccess> {
    state.zoom()?;
    let ctx = kernel::context::RequestContext::background("live.speaker_link");
    let mut tx = state.db().write(&ctx).await?;
    let session_id = lecture::seance_du_lien(&mut tx, jeton)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if !lecture::est_de_la_seance(&mut tx, session_id, speaker_id).await? {
        return Err(ApiError::not_found());
    }
    let acces = lien(&mut tx, session_id, speaker_id).await;
    tx.commit().await?;
    acces
}

pub async fn lien_pour_la_regie(
    state: &LiveState,
    perimetre: &Perimeter,
    session_id: Uuid,
    speaker_id: Uuid,
) -> Result<SpeakerZoomAccess> {
    meeting::autoriser(state, perimetre, session_id).await?;
    state.zoom()?;
    let ctx = kernel::context::RequestContext::background("live.speaker_link");
    let mut tx = state.db().write(&ctx).await?;
    let acces = lien(&mut tx, session_id, speaker_id).await;
    tx.commit().await?;
    acces
}
