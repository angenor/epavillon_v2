//! L'équipe et les membres d'une organisation (31/10).
//!
//! Le back-office ne faisait que regarder. Une organisation sans référent actif
//! n'avait alors plus personne pour accepter une demande : l'IFDD elle-même
//! s'y est retrouvée. L'équipe accepte donc une demande en fixant son rôle, et
//! change le rôle d'un membre actif. **Portée globale** : qui parle au nom d'une
//! organisation ne relève d'aucune édition.
//!
//! Deux règles demeurent : une invitation ne s'accepte jamais à la place de la
//! personne (écart n° 33), et retirer son rôle au dernier référent actif exige
//! une confirmation explicite (FR-041).

use contracts::org as evenements;
use kernel::auth::AdminScope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};

use crate::domain::admin::{AdminMembershipDecision, MembershipRoleChange, OrganizationWriteOutcome};
use crate::domain::ids::{MembershipId, OrganizationId, PersonId};
use crate::domain::membership::{Membership, MembershipRole, MembershipStatus};
use crate::jobs;
use crate::repo::memberships;
use crate::service::admin_write::recomposer;
use crate::service::membership::{emettre_approbation, emettre_revocation};
use crate::state::OrgState;

/// Accepter ou refuser une **demande**, à la place d'un référent absent.
pub async fn decide(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    acteur: PersonId,
    organisation: OrganizationId,
    id: MembershipId,
    demande: AdminMembershipDecision,
) -> Result<OrganizationWriteOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let Some(adhesion) = adhesion_de(&mut tx, organisation, id).await? else {
        return Ok(OrganizationWriteOutcome::NotFound);
    };
    if adhesion.is_pending_invitation() {
        return Err(ApiError::new(ErrorCode::OrgMembershipIsInvitation));
    }
    if adhesion.status != MembershipStatus::Pending {
        return Err(ApiError::new(ErrorCode::OrgMembershipNotPending));
    }

    if demande.approved {
        let role = demande.role.unwrap_or(adhesion.role);
        let approuvee = memberships::approve_request_as(&mut tx, id, acteur, role)
            .await?
            .ok_or_else(|| ApiError::new(ErrorCode::OrgMembershipNotPending))?;

        emettre_approbation(&mut tx, &approuvee).await?;
        jobs::emails::mettre_en_file_approbation(
            &mut tx,
            approuvee.id,
            approuvee.person_id.as_uuid(),
            approuvee.organization_id,
        )
        .await?;
    } else {
        let refusee = memberships::revoke(&mut tx, id)
            .await?
            .ok_or_else(|| ApiError::new(ErrorCode::OrgMembershipNotPending))?;
        emettre_revocation(&mut tx, &refusee, evenements::RevocationCause::Declined).await?;
    }

    jobs::planifier_apres_ecriture(&mut tx, state.config(), organisation).await?;
    tx.commit().await?;

    recomposer(state, perimetre, organisation).await
}

/// Changer le rôle d'un membre **actif** : désigner un référent, ou en retirer un.
pub async fn set_role(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    organisation: OrganizationId,
    id: MembershipId,
    demande: MembershipRoleChange,
) -> Result<OrganizationWriteOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let Some(adhesion) = adhesion_de(&mut tx, organisation, id).await? else {
        return Ok(OrganizationWriteOutcome::NotFound);
    };
    if adhesion.status != MembershipStatus::Active {
        return Err(ApiError::new(ErrorCode::OrgMembershipNotActive));
    }
    if adhesion.role == demande.role {
        tx.commit().await?;
        return recomposer(state, perimetre, organisation).await;
    }

    let dernier_referent = adhesion.role == MembershipRole::Manager
        && memberships::other_active_managers(&mut tx, organisation, id).await? == 0;
    if dernier_referent && !demande.confirm_last_manager {
        return Ok(OrganizationWriteOutcome::LastManager);
    }

    let modifiee = memberships::set_role(&mut tx, id, demande.role)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::OrgMembershipNotActive))?;

    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_MEMBERSHIP,
            aggregate_id: modifiee.id.as_uuid(),
            event_type: evenements::MEMBERSHIP_ROLE_CHANGED,
            payload: serde_json::to_value(evenements::MembershipRoleChanged {
                membership_id: modifiee.id.as_uuid(),
                organization_id: organisation.as_uuid(),
                person_id: modifiee.person_id.as_uuid(),
                previous_role: adhesion.role.as_str().to_owned(),
                role: modifiee.role.as_str().to_owned(),
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    jobs::planifier_apres_ecriture(&mut tx, state.config(), organisation).await?;
    tx.commit().await?;

    recomposer(state, perimetre, organisation).await
}

/// L'adhésion, si elle appartient bien à l'organisation du chemin : une URL
/// forgée ne touche pas une adhésion d'ailleurs.
async fn adhesion_de(
    conn: &mut sqlx::PgConnection,
    organisation: OrganizationId,
    id: MembershipId,
) -> Result<Option<Membership>> {
    Ok(memberships::by_id(&mut *conn, id)
        .await?
        .filter(|adhesion| adhesion.organization_id == organisation))
}
