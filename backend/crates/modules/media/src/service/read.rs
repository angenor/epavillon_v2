//! Les lectures d'un objet.
//!
//! # Un objet non servable n'est pas absent
//!
//! `GET /media/assets/{id}` rend un objet **en traitement**, **en échec** ou
//! **en quarantaine** avec son état, en 200. C'est ce qui permet à l'écran de
//! dire ce qui se passe : « en cours » et « en échec » se lisent tous les deux
//! « pas encore là », et les distinguer demande que l'API le dise.
//!
//! Seule la **suppression** rend 404 : là, l'objet n'existe plus.

use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::asset::{Asset, AssetProgress};
use crate::domain::variants;
use crate::repo::{assets, renditions};
use crate::state::MediaState;

/// Un objet et ses déclinaisons prêtes. L'adresse est **composée** par la base ;
/// aucune clé nue ne sort d'ici (FR-021).
pub async fn objet(state: &MediaState, asset_id: Uuid) -> Result<Asset> {
    assets::par_id(state.pool(), asset_id)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// L'avancement du traitement.
///
/// # Les déclinaisons ATTENDUES se comptent, elles ne s'annoncent pas
///
/// Trois tailles sont configurées, mais une image de 200 px n'en produit
/// aucune : elle est plus petite que la plus petite d'entre elles, et l'agrandir
/// n'ajouterait aucune information. Annoncer trois attendues laisserait
/// l'avancement bloqué à zéro sur trois pour toujours.
///
/// Tant que le relevé n'a pas eu lieu, la largeur est inconnue et le nombre
/// attendu vaut **zéro** : rien n'est encore connu comme attendu, et le dire
/// autrement serait inventer un dénominateur.
pub async fn avancement(state: &MediaState, asset_id: Uuid) -> Result<AssetProgress> {
    let etat = assets::etat_de_traitement(state.pool(), asset_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let attendues = match (etat.mime_type.starts_with("image/"), etat.width) {
        (true, Some(largeur)) => variants::attendues(largeur.max(0) as u32).len() as i64,
        _ => 0,
    };

    // Le motif vient de là où il est écrit : la file pour un travail mort, la
    // déclinaison pour une fabrication impossible. Aucune colonne de
    // `media.assets` ne le porte, et lui en ajouter une pour une trace serait
    // modifier le modèle.
    let last_error = match assets::motif_dechec(state.pool(), asset_id).await? {
        Some(motif) => Some(motif),
        None => renditions::dernier_echec(state.pool(), asset_id).await?,
    };

    Ok(AssetProgress {
        asset_id: etat.asset_id,
        status: etat.status,
        scan_verdict: etat.scan_verdict,
        scan_engine: etat.scan_engine,
        width: etat.width,
        height: etat.height,
        renditions_ready: renditions::compter_pretes(state.pool(), asset_id).await?,
        renditions_expected: attendues,
        last_error,
    })
}
