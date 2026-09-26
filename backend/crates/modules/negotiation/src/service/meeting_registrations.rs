//! S'inscrire, se désinscrire, relire ses inscriptions. Aucune vérification de
//! jauge ici : la base décide et le code traduit (principe VIII).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::meeting_registrations::{
    EtatDInscription, MeetingRegistrationState, MyMeetingRegistrations,
};
use crate::repo::{meeting_registrations as inscriptions, meetings};
use crate::service::documents::a_lacces;
use crate::state::NegotiationState;

fn inconnue() -> ApiError {
    ApiError::new(ErrorCode::NegotiationMeetingUnknown)
}

pub async fn mes_inscriptions(
    state: &NegotiationState,
    person_id: Uuid,
    edition: &str,
) -> Result<MyMeetingRegistrations> {
    let acces = a_lacces(state, Some(person_id)).await?;
    let mut conn = state.pool().acquire().await?;
    let edition = meetings::edition(&mut conn, edition.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;
    let video = if acces {
        inscriptions::liens(&mut conn, person_id, edition.event_id).await?
    } else {
        Vec::new()
    };
    Ok(MyMeetingRegistrations {
        registrations: inscriptions::mes_inscriptions(&mut conn, person_id, edition.event_id)
            .await?,
        video,
    })
}

/// Même `client_ref` que la ligne, ou déjà inscrite : l'état courant, rien
/// d'écrit. Ligne désinscrite et référence neuve : réinscription (R5).
pub async fn inscrire(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    meeting_id: Uuid,
    client_ref: Uuid,
) -> Result<MeetingRegistrationState> {
    if !a_lacces(state, Some(person_id)).await? {
        return Err(ApiError::new(ErrorCode::NegotiationMeetingForbidden));
    }
    let mut tx = state.db().write(ctx).await?;
    inscriptions::verrouiller(&mut tx, meeting_id)
        .await?
        .ok_or_else(inconnue)?;
    let etat = match inscriptions::ligne(&mut tx, meeting_id, person_id).await? {
        Some(l) if l.client_ref == Some(client_ref) || l.status != EtatDInscription::Cancelled => {
            tx.rollback().await?;
            return Ok(l.etat());
        }
        Some(_) => inscriptions::reinscrire(&mut tx, meeting_id, person_id, client_ref).await?,
        None => inscriptions::inserer(&mut tx, meeting_id, person_id, client_ref).await?,
    };
    tx.commit().await?;
    Ok(etat)
}

/// Idempotent. Refusée après le début — la base, elle, laisse faire. La place
/// libérée passe à la liste d'attente dans la même transaction.
pub async fn desinscrire(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    meeting_id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let reunion = inscriptions::verrouiller(&mut tx, meeting_id)
        .await?
        .ok_or_else(inconnue)?;
    let active = inscriptions::ligne(&mut tx, meeting_id, person_id)
        .await?
        .is_some_and(|l| l.status != EtatDInscription::Cancelled);
    if !active {
        tx.rollback().await?;
        return Ok(());
    }
    if reunion.commencee {
        tx.rollback().await?;
        return Err(ApiError::new(ErrorCode::NegotiationMeetingUnavailable));
    }
    inscriptions::desinscrire(&mut tx, meeting_id, person_id).await?;
    if reunion.programmee {
        let promues = inscriptions::promouvoir(&mut tx, meeting_id).await?;
        prevenir_des_promotions(meeting_id, &promues);
    }
    tx.commit().await?;
    Ok(())
}

/// T017 : l'avis et le courriel de chaque personne promue partiront d'ici, dans
/// la transaction de la promotion. Rien n'est émis à la phase 2.
fn prevenir_des_promotions(_meeting_id: Uuid, _promues: &[Uuid]) {}
