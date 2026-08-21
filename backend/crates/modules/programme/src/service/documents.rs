//! Les pièces du dossier — **rattacher et détacher, jamais téléverser ni
//! détruire**.
//!
//! # La frontière, et pourquoi elle tient
//!
//! `media.assets` appartient au module Médias : c'est lui qui pose l'objet,
//! l'analyse, le sert et le purge. Ce module **désigne** un objet déjà stocké
//! et le rattache à un dossier. Le téléversement réel appartient à B6.
//!
//! La conséquence pratique est un refus, pas une prudence : **le détachement
//! ne détruit pas l'objet**. Un même fichier peut être rattaché à deux
//! dossiers, et le détruire ici effacerait la pièce d'un autre sans le savoir.
//!
//! # L'accès est celui du dossier — les deux voies
//!
//! L'organisation joint ses pièces, l'IFDD en ajoute aussi. Aucune des deux
//! n'est privilégiée, et le refus est celui d'un dossier inexistant.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::draft;
use crate::domain::ids::ProposalId;
use crate::repo::documents::{NouvellePiece, Piece, PieceDuDossier};
use crate::repo::{cross, documents};
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// Ce que le rattachement reçoit.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AttachDocumentPayload {
    /// **Un objet déjà stocké.** Le téléversement appartient à B6.
    pub asset_id: Uuid,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub document_type_code: Option<String>,
    /// Pièce visible du public une fois l'activité publiée, ou **interne au
    /// dossier d'évaluation**. Le défaut est interne : une pièce jointe à un
    /// dossier n'est pas publique parce qu'on a oublié de le dire.
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub sort_order: i16,
}

/// Les pièces d'un dossier, **après contrôle d'accès**.
pub async fn lister(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<Vec<PieceDuDossier>> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    documents::du_dossier(state.pool(), dossier).await
}

/// Rattacher un objet stocké au dossier.
pub async fn rattacher(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: AttachDocumentPayload,
) -> Result<Piece> {
    perimeter::acces_au_dossier(state.pool(), acteur, dossier).await?;

    // **L'objet est vérifié avant l'écriture, pour nommer le champ.** La clé
    // étrangère refuserait aussi, mais son message ne dirait pas lequel.
    let objet = cross::objet_stocke(state.pool(), payload.asset_id)
        .await?
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ProposalUnknownReference,
                "Cet objet stocké est inconnu, ou a été supprimé.",
            )
            .field("asset_id")
        })?;

    // Le titre par défaut est le nom du fichier d'origine : une pièce sans
    // titre s'affiche « Document » dans une liste, et personne ne sait laquelle
    // ouvrir.
    let titre = match payload.title.trim() {
        "" => objet
            .original_filename
            .clone()
            .unwrap_or_else(|| "Pièce jointe".to_owned()),
        saisi => saisi.to_owned(),
    };

    let mut tx = state.db().write(ctx).await?;
    let piece = documents::rattacher(
        &mut tx,
        dossier,
        acteur,
        &NouvellePiece {
            asset_id: payload.asset_id,
            title: draft::i18n_obligatoire(&titre),
            document_type_code: payload.document_type_code.as_deref(),
            is_public: payload.is_public,
            sort_order: payload.sort_order,
        },
    )
    .await?;
    tx.commit().await?;

    Ok(piece)
}

/// Détacher une pièce — **l'objet stocké demeure**.
pub async fn detacher(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    piece: Uuid,
) -> Result<()> {
    perimeter::acces_au_dossier(state.pool(), acteur, dossier).await?;

    let mut tx = state.db().write(ctx).await?;
    let detachee = documents::detacher(&mut tx, dossier, piece).await?;
    tx.commit().await?;

    if detachee {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}
