//! Les écritures de la fiche : le sceau, un domaine, une dénomination, et la
//! fiche elle-même (31/10).
//!
//! **Chacune rend la fiche entière recomposée.** Vérifier un domaine change le
//! score de confiance, qui change le rang de la fiche dans la liste ; poser le
//! sceau change ce que la file des doublons affiche. Rendre le seul objet
//! modifié laisserait trois panneaux afficher des valeurs fausses jusqu'au
//! prochain rechargement.
//!
//! **Les refus de la base sont traduits, jamais réimplémentés** (principe VIII).

use contracts::org as evenements;
use kernel::auth::AdminScope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};

use crate::domain::admin::{
    DomainVerification, NameConfirmation, OrganizationProfile, OrganizationRef,
    OrganizationVerification, OrganizationWriteOutcome,
};
use crate::domain::ids::{OrganizationDomainId, OrganizationId, OrganizationNameId, PersonId};
use crate::jobs;
use crate::repo::{admin_detail, domains, names, organizations};
use crate::service::admin_detail as fiche;
use crate::state::OrgState;

/// Pose ou retire le sceau.
///
/// **Poser le sceau admet la fiche du même geste** : une organisation reconnue
/// par l'IFDD n'a plus de raison de rester en attente de rapprochement. Le
/// retirer, en revanche, **ne change pas le statut** — la fiche reste active,
/// elle cesse d'être certifiée.
pub async fn set_verification(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    acteur: PersonId,
    id: OrganizationId,
    demande: OrganizationVerification,
) -> Result<OrganizationWriteOutcome> {
    if !fiche::dans_le_perimetre(state.pool(), perimetre, id).await? {
        return Ok(OrganizationWriteOutcome::NotFound);
    }

    let mut tx = state.db().write(ctx).await?;

    if demande.verified {
        let Some(instant) = organizations::set_verified(&mut tx, id, acteur).await? else {
            return Ok(OrganizationWriteOutcome::NotFound);
        };

        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_ORGANIZATION,
                aggregate_id: id.as_uuid(),
                event_type: evenements::ORGANIZATION_VERIFIED,
                payload: serde_json::to_value(evenements::OrganizationVerified {
                    organization_id: id.as_uuid(),
                    verified_at: instant,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    } else {
        if !organizations::clear_verified(&mut tx, id).await? {
            return Ok(OrganizationWriteOutcome::NotFound);
        }

        // **Deux événements et non un seul portant un booléen** : un événement
        // nommé « vérifiée » portant « non » est un mensonge que personne ne
        // relit correctement.
        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_ORGANIZATION,
                aggregate_id: id.as_uuid(),
                event_type: evenements::ORGANIZATION_UNVERIFIED,
                payload: serde_json::to_value(evenements::OrganizationUnverified {
                    organization_id: id.as_uuid(),
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    }

    jobs::planifier_apres_ecriture(&mut tx, state.config(), id).await?;
    tx.commit().await?;

    recomposer(state, perimetre, id).await
}

/// Vérifie manuellement un domaine, et règle son rattachement automatique.
pub async fn set_domain(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    id: OrganizationId,
    domain_id: OrganizationDomainId,
    demande: DomainVerification,
) -> Result<OrganizationWriteOutcome> {
    if !fiche::dans_le_perimetre(state.pool(), perimetre, id).await? {
        return Ok(OrganizationWriteOutcome::NotFound);
    }

    let mut tx = state.db().write(ctx).await?;

    let ecrit =
        domains::set_verification(&mut tx, id, domain_id, demande.verified, demande.auto_join)
            .await;

    match ecrit {
        Ok(true) => {}
        Ok(false) => return Ok(OrganizationWriteOutcome::NotFound),
        Err(e) if contrainte(&e, "ux_organization_domains_verified") => {
            // **La transaction est rendue AVANT de lire la fiche en cause** :
            // une violation de contrainte abandonne la transaction, et toute
            // lecture qui suivrait y échouerait sur « current transaction is
            // aborted » — une erreur interne à la place du refus attendu.
            tx.rollback().await?;

            // **Le refus doit nommer la fiche qui détient le domaine.** Sans ce
            // nom, il est incompréhensible : « ce domaine est déjà pris »
            // n'apprend rien à qui ne sait pas par qui.
            let mut conn = state.pool().acquire().await?;
            let detenteur = domains::holder_of_verified(&mut *conn, domain_id).await?;

            return Ok(match detenteur {
                Some((organization_id, legal_name)) => OrganizationWriteOutcome::DomainTaken {
                    conflict_with: OrganizationRef {
                        organization_id,
                        legal_name,
                    },
                },
                // La contrainte a refusé et personne ne détient le domaine : la
                // ligne concurrente a été défaite entre-temps. Rien à dire de
                // plus honnête que « réessayez ».
                None => OrganizationWriteOutcome::NotFound,
            });
        }
        Err(e) if contrainte(&e, "ck_domain_autojoin_requires_verification") => {
            return Err(ApiError::new(ErrorCode::OrgDomainVerificationRequired).field("auto_join"));
        }
        Err(e) => return Err(e),
    }

    jobs::planifier_apres_ecriture(&mut tx, state.config(), id).await?;
    tx.commit().await?;

    recomposer(state, perimetre, id).await
}

/// Confirme ou déconfirme une dénomination.
pub async fn set_name_confirmation(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    id: OrganizationId,
    name_id: OrganizationNameId,
    demande: NameConfirmation,
) -> Result<OrganizationWriteOutcome> {
    if !fiche::dans_le_perimetre(state.pool(), perimetre, id).await? {
        return Ok(OrganizationWriteOutcome::NotFound);
    }

    // **Une dénomination posée par la base ne se retire pas.** Le trigger la
    // repose à la première modification de la fiche : la déconfirmer laisserait
    // croire à un geste qui n'a aucun effet durable.
    match names::est_derivee(state.pool(), name_id).await? {
        None => return Ok(OrganizationWriteOutcome::NotFound),
        Some(true) if !demande.is_confirmed => {
            return Err(ApiError::new(ErrorCode::OrgNameIsDerived).field("is_confirmed"));
        }
        Some(_) => {}
    }

    let mut tx = state.db().write(ctx).await?;
    if !names::set_confirmed(&mut tx, id, name_id, demande.is_confirmed).await? {
        return Ok(OrganizationWriteOutcome::NotFound);
    }

    // Une dénomination confirmée ne change pas le score de confiance — la
    // fonction du modèle ne la compte pas — mais elle change ce que la fiche de
    // performance affiche. La projection suit donc, le score aussi : le travail
    // n'écrit que si la valeur bouge.
    jobs::planifier_apres_ecriture(&mut tx, state.config(), id).await?;
    tx.commit().await?;

    recomposer(state, perimetre, id).await
}

/// Corrige la fiche : nom, sigle et sa notoriété, hôte du pavillon, type, pays, site (31/10).
///
/// La fiche est partagée par tous ses dossiers et ses membres : la corriger ici
/// la corrige partout. L'ancien nom et l'ancien sigle restent cherchables
/// (`tg_organizations_sync_names`). Un nom déjà porté par une autre fiche du
/// même pays est refusé en la nommant, comme un domaine déjà détenu.
pub async fn set_profile(
    state: &OrgState,
    ctx: &RequestContext,
    perimetre: &AdminScope,
    id: OrganizationId,
    demande: OrganizationProfile,
) -> Result<OrganizationWriteOutcome> {
    if !fiche::dans_le_perimetre(state.pool(), perimetre, id).await? {
        return Ok(OrganizationWriteOutcome::NotFound);
    }

    let nom = demande.legal_name.trim();
    if nom.chars().count() < 2 {
        return Err(ApiError::validation(
            "Le nom de l'organisation compte au moins deux caractères.",
            "legal_name",
        ));
    }
    let sigle = demande
        .acronym
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let site = demande
        .website
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut tx = state.db().write(ctx).await?;

    if !organizations::type_is_active(&mut tx, &demande.organization_type_code).await? {
        return Err(ApiError::validation(
            "Choisissez un type d'organisation dans la liste.",
            "organization_type_code",
        ));
    }

    let changes = match organizations::update_profile(
        &mut tx,
        id,
        nom,
        sigle,
        demande.acronym_well_known,
        demande.hosts_pavilion,
        &demande.organization_type_code,
        demande.country_id,
        site,
    )
    .await
    {
        Ok(Some(changes)) => changes,
        Ok(None) => return Ok(OrganizationWriteOutcome::NotFound),
        Err(erreur) if contrainte(&erreur, "ux_organizations_name_country") => {
            drop(tx);
            return Ok(match homonyme(state, id, nom, demande.country_id).await? {
                Some(conflict_with) => OrganizationWriteOutcome::NameTaken { conflict_with },
                None => return Err(erreur),
            });
        }
        Err(erreur) => return Err(erreur),
    };

    if !changes.is_empty() {
        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_ORGANIZATION,
                aggregate_id: id.as_uuid(),
                event_type: evenements::ORGANIZATION_UPDATED,
                payload: serde_json::to_value(evenements::OrganizationUpdated {
                    organization_id: id.as_uuid(),
                    changed_fields: changes,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
        jobs::planifier_apres_ecriture(&mut tx, state.config(), id).await?;
    }
    tx.commit().await?;

    recomposer(state, perimetre, id).await
}

/// La fiche vivante qui porte déjà ce nom dans ce pays.
async fn homonyme(
    state: &OrgState,
    sauf: OrganizationId,
    nom: &str,
    pays: Option<uuid::Uuid>,
) -> Result<Option<OrganizationRef>> {
    let ligne = sqlx::query!(
        "SELECT id, legal_name FROM org.organizations
          WHERE legal_name_normalized = platform.normalize_label($1)
            AND COALESCE(country_id, '00000000-0000-0000-0000-000000000000'::uuid)
              = COALESCE($2, '00000000-0000-0000-0000-000000000000'::uuid)
            AND status IN ('candidate', 'active') AND id <> $3
          LIMIT 1",
        nom,
        pays,
        sauf.as_uuid()
    )
    .fetch_optional(state.pool())
    .await?;

    Ok(ligne.map(|l| OrganizationRef {
        organization_id: OrganizationId(l.id),
        legal_name: l.legal_name,
    }))
}

/// La fiche entière, relue après l'écriture.
pub(crate) async fn recomposer(
    state: &OrgState,
    perimetre: &AdminScope,
    id: OrganizationId,
) -> Result<OrganizationWriteOutcome> {
    Ok(match fiche::detail(state.pool(), perimetre, id).await? {
        Some(detail) => OrganizationWriteOutcome::Saved {
            detail: Box::new(detail),
        },
        None => OrganizationWriteOutcome::NotFound,
    })
}

/// Le nom de la contrainte violée se lit dans le détail technique, que le noyau
/// y dépose et qui ne franchit jamais la réponse HTTP.
fn contrainte(erreur: &ApiError, nom: &str) -> bool {
    erreur.detail.as_deref().is_some_and(|d| d.contains(nom))
}

/// Le nom d'une fiche, pour un renvoi. Rendu public parce que la fusion s'en
/// sert aussi.
pub async fn nom_de(state: &OrgState, id: OrganizationId) -> Result<Option<String>> {
    let mut conn = state.pool().acquire().await?;
    admin_detail::nom_de(&mut conn, id.as_uuid()).await
}
