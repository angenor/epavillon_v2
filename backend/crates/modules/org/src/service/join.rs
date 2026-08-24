//! Rejoindre une organisation.
//!
//! **C'est le domaine qui décide, pas la volonté de la personne.** Une adresse
//! sur un domaine vérifié et ouvert au rattachement automatique entre d'office ;
//! toute autre attend qu'un référent tranche. `pending` n'est pas un échec —
//! c'est le fonctionnement normal, et l'écran doit le dire plutôt que de laisser
//! croire que tout est réglé.

use contracts::org as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};

use crate::domain::ids::{OrganizationId, PersonId};
use crate::domain::membership::{JoinOrganization, JoinOutcome, MembershipRole};
use crate::jobs;
use crate::repo::memberships::RequestOutcome;
use crate::repo::{domains, memberships, organizations};
use crate::state::OrgState;

/// Demande de rattachement.
///
/// L'organisation visée est **résolue** : rejoindre une fiche absorbée mène à la
/// fiche vivante (FR-024). `org.resolve_organization()` existe pour cela, et le
/// trigger garantit qu'il n'y a jamais de chaîne à remonter.
pub async fn join(
    state: &OrgState,
    ctx: &RequestContext,
    person_id: PersonId,
    visee: OrganizationId,
    demande: JoinOrganization,
) -> Result<JoinOutcome> {
    // Avant toute écriture : une adhésion qui deviendrait active sans fonction
    // serait refusée par la base, et le refus arriverait alors sans nommer le
    // champ fautif.
    let fonction = crate::domain::membership::fonction_declaree(demande.job_title.as_deref())?;

    let mut tx = state.db().write(ctx).await?;

    let Some(cible) = organizations::resolve(&mut *tx, visee).await? else {
        return Err(ApiError::not_found());
    };

    let Some(organisation) = organizations::by_id(&mut *tx, cible).await? else {
        return Err(ApiError::not_found());
    };

    // Relu **dans la transaction** : un domaine vérifié entre l'ouverture de
    // l'écran et le clic changerait sinon l'issue sans que rien ne le relise.
    let auto = domains::auto_join_applies(&mut tx, cible, person_id.as_uuid()).await?;

    let (issue, adhesion) = memberships::request(
        &mut tx,
        cible,
        person_id,
        MembershipRole::Member,
        Some(fonction.as_str()),
        auto,
    )
    .await?;

    if issue == RequestOutcome::AlreadyThere {
        let statut = adhesion.status;
        tx.commit().await?;
        return Ok(JoinOutcome::AlreadyMember {
            organization: Box::new(organisation),
            membership_status: statut,
        });
    }

    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_MEMBERSHIP,
            aggregate_id: adhesion.id.as_uuid(),
            event_type: evenements::MEMBERSHIP_REQUESTED,
            payload: serde_json::to_value(evenements::MembershipRequested {
                membership_id: adhesion.id.as_uuid(),
                organization_id: cible.as_uuid(),
                person_id: person_id.as_uuid(),
                direction: evenements::MembershipDirection::Requested,
                auto_joined: auto,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    if auto {
        // Une adhésion active de plus change le score de confiance et la fiche
        // de performance. Les deux travaux naissent **dans la transaction** : si
        // elle est annulée, ils ne naissent pas.
        jobs::planifier_apres_ecriture(&mut tx, state.config(), cible).await?;
    } else {
        // Les référents doivent savoir qu'une demande les attend.
        jobs::emails::mettre_en_file_demande(&mut tx, adhesion.id, cible).await?;
    }

    let membership_id = adhesion.id;
    tx.commit().await?;

    Ok(if auto {
        JoinOutcome::Joined {
            membership_id,
            organization: Box::new(organisation),
        }
    } else {
        JoinOutcome::Pending {
            membership_id,
            organization: Box::new(organisation),
        }
    })
}
