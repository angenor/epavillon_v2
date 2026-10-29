//! L'atelier de l'image de couverture d'un dossier.
//!
//! Le site dessine l'image et la rattache lui-même (`cover`, module média) ;
//! ici ne vivent que les choix, pour rouvrir l'atelier là où on l'a laissé, et
//! la lecture de tout ce qu'il compose. La photo de départ (`cover_source`) se
//! pose aussi par le module média, sous la garde du dossier.
//!
//! Lecture : celle du reste de la fiche. Écriture : celle du jour J
//! (organisation porteuse ou équipe qui décide), sur tout dossier encore en
//! lice, même après la fin de l'édition (arbitrage de la phase 1 : les images
//! restent modifiables).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::cover_design::{self, CoverDesignOptions, CoverStudioContext};
use crate::domain::ids::ProposalId;
use crate::repo::cover_design as repo;
use crate::service::{day_plan, perimeter};
use crate::state::ProgrammeState;

/// `SaveCoverDesignPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SaveCoverDesignPayload {
    pub template_code: String,
    pub accent: String,
    #[serde(default)]
    pub options: CoverDesignOptions,
}

pub async fn lire(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<CoverStudioContext> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    composer(state, dossier).await
}

pub async fn enregistrer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveCoverDesignPayload,
) -> Result<CoverStudioContext> {
    let ligne = day_plan::porteuse_ou_equipe(state, acteur, dossier).await?;
    let clos = match ligne.status.as_str() {
        "rejected" => {
            Some("Ce dossier n'a pas été retenu : son image de couverture ne se modifie plus.")
        }
        "withdrawn" => {
            Some("Ce dossier a été retiré : son image de couverture ne se modifie plus.")
        }
        "cancelled" => {
            Some("Cette activité a été annulée : son image de couverture ne se modifie plus.")
        }
        _ => None,
    };
    if let Some(message) = clos {
        return Err(ApiError::with_message(
            ErrorCode::ProposalNotEditable,
            message,
        ));
    }

    let template_code = payload.template_code.trim();
    if !cover_design::code_valide(template_code) {
        return Err(ApiError::validation(
            "Choisissez un modèle d'image dans la liste.",
            "template_code",
        ));
    }
    let accent = payload.accent.trim();
    if !cover_design::code_valide(accent) {
        return Err(ApiError::validation(
            "Choisissez une couleur dans la liste.",
            "accent",
        ));
    }

    let mut tx = state.db().write(ctx).await?;
    repo::ecrire(
        &mut tx,
        dossier,
        acteur,
        template_code,
        accent,
        payload.options,
    )
    .await?;
    tx.commit().await?;

    composer(state, dossier).await
}

async fn composer(state: &ProgrammeState, dossier: ProposalId) -> Result<CoverStudioContext> {
    let pool = state.pool();
    let images = repo::images(pool, dossier).await?;
    Ok(CoverStudioContext {
        design: repo::choix(pool, dossier).await?,
        source_photo: images.source_photo,
        edition_banner: images.edition_banner,
        edition_thumbnail: images.edition_thumbnail,
        organization_logo: images.organization_logo,
    })
}
