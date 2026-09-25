//! Retours et signalements du lecteur connecté. Ni l'un ni l'autre ne touche
//! l'entrée (FR-018).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::savoir_retours::{
    motif_du_retour, signalement_valide, FaqFeedback, FaqFeedbackInput, FaqReportInput,
    FaqReportReceipt, MyFaqFeedback, DEPASSEE, PLAFOND_SIGNALEMENTS,
};
use crate::repo::savoir_retours as repo;
use crate::state::NegotiationState;

fn introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationFaqNotFound)
}

pub async fn voter(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    entry_id: Uuid,
    entree: &FaqFeedbackInput,
) -> Result<FaqFeedback> {
    let motif = motif_du_retour(entree)?;
    let mut tx = state.db().write(ctx).await?;
    if !repo::servie(&mut tx, entry_id).await? {
        return Err(introuvable());
    }
    let voix = repo::voter(&mut tx, entry_id, personne, entree.helpful, motif).await?;
    if motif == Some(DEPASSEE) {
        repo::signaler_depuis_le_retour(&mut tx, entry_id, personne).await?;
    }
    tx.commit().await?;
    Ok(voix)
}

/// Rejoué avec le même `client_ref` : le reçu d'origine, et `false`.
pub async fn signaler(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    entry_id: Uuid,
    entree: &FaqReportInput,
) -> Result<(FaqReportReceipt, bool)> {
    let mut tx = state.db().write(ctx).await?;
    repo::verrouiller(&mut tx, personne).await?;
    if let Some(recu) = repo::recu(&mut tx, personne, entree.client_ref).await? {
        return Ok((recu, false));
    }
    let (motifs, details) = signalement_valide(entree)?;
    if !repo::servie(&mut tx, entry_id).await? {
        return Err(introuvable());
    }
    if repo::envoyes_aujourdhui(&mut tx, personne).await? >= PLAFOND_SIGNALEMENTS {
        return Err(ApiError::new(ErrorCode::NegotiationReportLimit));
    }
    let recu = repo::signaler(
        &mut tx,
        entry_id,
        personne,
        entree.client_ref,
        &motifs,
        details.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok((recu, true))
}

pub async fn mes_voix(state: &NegotiationState, personne: Uuid) -> Result<(MyFaqFeedback, String)> {
    let mut conn = state.pool().acquire().await?;
    let feedback = repo::mes_voix(&mut conn, personne).await?;
    let empreinte = kernel::empreinte::de(
        &feedback
            .iter()
            .map(|v| {
                format!(
                    "{}:{}:{}",
                    v.entry_id,
                    v.helpful,
                    v.missing_reason.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok((MyFaqFeedback { feedback }, empreinte))
}
