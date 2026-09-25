//! « Mon agenda ». Un rappel sur une session annulée s'enregistre désarmé
//! (FR-035) ; une annulée ne s'ajoute pas, mais reste réglable si elle y est.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::agenda::MyAgenda;
use crate::repo::agenda;
use crate::state::NegotiationState;

pub async fn mon_agenda(state: &NegotiationState, person_id: Uuid) -> Result<MyAgenda> {
    let mut conn = state.pool().acquire().await?;
    Ok(MyAgenda {
        entries: agenda::lister(&mut conn, person_id).await?,
        network_entries: agenda::lister_reseau(&mut conn, person_id).await?,
    })
}

pub async fn poser(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    session_id: Uuid,
    rappel: bool,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let visee = agenda::session(&mut tx, person_id, session_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationSessionUnknown))?;
    if visee.annulee && !visee.dans_lagenda {
        tx.rollback().await?;
        return Err(ApiError::new(ErrorCode::NegotiationSessionCancelled));
    }
    agenda::poser(&mut tx, person_id, session_id, rappel && !visee.annulee).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn retirer(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    session_id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    agenda::retirer(&mut tx, person_id, session_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Une réunion non annoncée : publiée et non retirée, sinon inconnue.
pub async fn poser_reunion(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    network_meeting_id: Uuid,
    rappel: bool,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if !agenda::reunion_affichee(&mut tx, network_meeting_id).await? {
        return Err(ApiError::new(ErrorCode::NegotiationSessionUnknown));
    }
    agenda::poser_reunion(&mut tx, person_id, network_meeting_id, rappel).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn retirer_reunion(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    network_meeting_id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    agenda::retirer_reunion(&mut tx, person_id, network_meeting_id).await?;
    tx.commit().await?;
    Ok(())
}
