//! Rattacher un dossier à une autre organisation porteuse, ou à un autre
//! déposant (01/11) : le dossier a été déposé au nom de la mauvaise fiche, ou
//! la personne qui le suit a changé.
//!
//! Gardé comme la correction par l'équipe : périmètre de l'édition, puis
//! `programme.proposal.edit` sur elle. L'audit du dossier trace le changement
//! (`tg_proposals_audit`, acteur posé par la porte d'écriture) ; le modèle ne
//! prévoit pas d'événement de domaine pour ces deux colonnes, aucun n'est émis.
//!
//! **La porteuse se change par `proposals.organization_id`, jamais par la
//! table des organisations du dossier** : `tg_sync_proposal_lead_organization`
//! retire l'ancienne ligne `lead` et pose la nouvelle. Les co-organisatrices ne
//! bougent pas, sauf celle qui devient porteuse (sa ligne passe `lead`). Les
//! séances suivent, et leur propre déclencheur resynchronise leur porteuse.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::domain::permissions::PROPOSAL_EDIT;
use crate::repo::cross;
use crate::repo::proposals::{self, Fiche};
use crate::service::perimeter;
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeLeadOrganizationPayload {
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeSubmitterPayload {
    pub person_id: Uuid,
}

/// Périmètre, permission, puis l'état du dossier : un brouillon appartient à
/// l'organisation qui l'écrit.
async fn garder(
    state: &ProgrammeState,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
) -> Result<Fiche> {
    let edition = perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        perimeter::Cible::Dossier(dossier),
    )
    .await?;
    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        PROPOSAL_EDIT,
        kernel::auth::Scope::Event(edition.as_uuid()),
    )
    .await?;

    let fiche = proposals::fiche(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if fiche.status == "draft" {
        return Err(ApiError::with_message(
            ErrorCode::ProposalNotEditable,
            "Ce dossier est encore en brouillon : seule son organisation le complète.",
        ));
    }
    Ok(fiche)
}

pub async fn changer_de_porteuse(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
    payload: ChangeLeadOrganizationPayload,
) -> Result<Fiche> {
    let avant = garder(state, perimetre, dossier).await?;
    if avant.organization_id == payload.organization_id {
        return Ok(avant);
    }

    let mut tx = state.db().write(ctx).await?;
    if !cross::organisation_peut_porter(&mut *tx, payload.organization_id).await? {
        return Err(ApiError::with_message(
            ErrorCode::ProposalUnknownReference,
            "Cette organisation n'existe pas, ou ne peut plus porter de dossier (fiche absorbée, archivée ou refusée).",
        )
        .field("organization_id"));
    }

    sqlx::query!(
        "UPDATE programme.proposals SET organization_id = $2 WHERE id = $1",
        dossier.as_uuid(),
        payload.organization_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE programme.sessions SET organization_id = $3
          WHERE proposal_id = $1 AND organization_id IS NOT DISTINCT FROM $2",
        dossier.as_uuid(),
        avant.organization_id,
        payload.organization_id
    )
    .execute(&mut *tx)
    .await?;

    let apres = proposals::fiche(&mut *tx, dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;
    Ok(apres)
}

pub async fn changer_de_deposant(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
    payload: ChangeSubmitterPayload,
) -> Result<Fiche> {
    let avant = garder(state, perimetre, dossier).await?;
    if avant.submitted_by == payload.person_id {
        return Ok(avant);
    }

    let mut tx = state.db().write(ctx).await?;
    if !cross::personne_peut_deposer(&mut *tx, payload.person_id).await? {
        return Err(ApiError::with_message(
            ErrorCode::ProposalUnknownReference,
            "Cette personne n'existe pas, ou son compte n'est pas actif.",
        )
        .field("person_id"));
    }

    sqlx::query!(
        "UPDATE programme.proposals SET submitted_by = $2 WHERE id = $1",
        dossier.as_uuid(),
        payload.person_id
    )
    .execute(&mut *tx)
    .await?;

    let apres = proposals::fiche(&mut *tx, dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;
    Ok(apres)
}
