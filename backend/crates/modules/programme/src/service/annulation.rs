//! L'organisation annule elle-même son activité retenue (04/11/2026).
//!
//! Même transition que l'annulation par l'équipe (`accepted -> cancelled`),
//! mêmes effets (créneaux pressentis libérés, séances publiées annulées, ce qui
//! prévient les inscrits et le direct par `programme.session.cancelled`), à deux
//! différences près :
//!
//! - **le motif est exigé, et il reste privé** : il vit dans `decision_reason`
//!   et le journal du dossier, que l'organisation et l'équipe lisent ; les
//!   séances s'annulent sans motif, la page publique n'en affiche donc aucun ;
//! - **l'équipe est prévenue aussitôt** : le service émet
//!   `programme.proposal.cancelled_by_organization`, auquel `engagement` répond
//!   par un courriel. Ce n'est pas un doublon de `programme.proposal.cancelled`,
//!   que le déclencheur émet pour toute annulation sans savoir qui l'a faite.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::annulation::{self, Refus};
use crate::domain::ids::ProposalId;
use crate::domain::transitions::ProposalStatus;
use crate::repo::transitions::{self as journal, LigneDeJournal};
use crate::service::perimeter::{self, Acces};
use crate::service::transition::{self, Issue};
use crate::state::ProgrammeState;

/// `CancelProposalPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CancelProposalPayload {
    #[serde(default)]
    pub reason: String,
}

pub async fn annuler_par_lorganisation(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    motif: &str,
) -> Result<LigneDeJournal> {
    // La règle des autres gestes de l'organisation sur son dossier
    // (`domain/ownership::peut_agir_sur`). Un administrateur non membre passe
    // par la fiche du comité : ici, il reçoit le même 404 qu'un inconnu.
    let (_, acces) = perimeter::acces_au_dossier(state.pool(), acteur, dossier).await?;
    if acces != Acces::Organisation {
        return Err(ApiError::not_found());
    }

    let mut tx = state.db().write(ctx).await?;

    // Verrou sur le dossier : deux annulations simultanées se sérialisent, et la
    // seconde lit `cancelled`.
    let etat = sqlx::query!(
        r#"SELECT p.status::text AS "status!", p.reference_code, p.event_id, p.organization_id,
                  EXISTS (
                      SELECT 1 FROM programme.sessions s
                       WHERE (s.proposal_id = p.id
                              OR s.id IN (SELECT sp.session_id FROM programme.session_proposals sp
                                           WHERE sp.proposal_id = p.id))
                         AND (s.status IN ('live', 'completed')
                              OR (s.status IN ('planned', 'scheduled') AND s.starts_at <= now()))
                  ) AS "commencee!"
             FROM programme.proposals p
            WHERE p.id = $1 AND p.deleted_at IS NULL
              FOR UPDATE OF p"#,
        dossier.as_uuid()
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::not_found)?;

    annulation::verifier(&etat.status, etat.commencee, motif)?;
    let motif = motif.trim();

    if let Err(erreur) =
        transition::ecrire_letat(&mut tx, dossier, ProposalStatus::Cancelled, Some(motif)).await
    {
        tx.rollback().await?;
        return Err(match transition::traduire(&erreur)? {
            Issue::TransitionInterdite(message) => {
                ApiError::with_message(ErrorCode::Conflict, message)
            }
            Issue::MotifExige => Refus::MotifManquant.into(),
            Issue::Appliquee(_) => ApiError::internal("issue impossible d'une écriture refusée"),
        });
    }

    // Aucun motif public : celui de l'organisation ne quitte pas le dossier.
    transition::appliquer_les_effets(&mut tx, dossier, ProposalStatus::Cancelled, None).await?;

    let ligne = journal::derniere(&mut *tx, dossier)
        .await?
        .ok_or_else(|| ApiError::internal("le déclencheur n'a pas journalisé la transition"))?;

    let charge = contracts::programme::ProposalCancelledByOrganization {
        proposal_id: dossier.as_uuid(),
        reference_code: etat.reference_code,
        event_id: etat.event_id,
        organization_id: etat.organization_id,
        transition_id: ligne.id,
    };
    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: contracts::programme::AGGREGATE_SCHEMA,
            aggregate_type: contracts::programme::AGGREGATE_PROPOSAL,
            aggregate_id: dossier.as_uuid(),
            event_type: contracts::programme::PROPOSAL_CANCELLED_BY_ORGANIZATION,
            payload: serde_json::to_value(&charge).map_err(ApiError::internal)?,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(ligne)
}
