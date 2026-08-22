//! Événements du module `media`.
//!
//! # Deux des trois événements de ce schéma N'ONT PAS leur place ici
//!
//! `media.asset.uploaded` est émis par `media.tg_enqueue_processing()`, à
//! chaque insertion dans `media.assets`. `media.asset.purge_scheduled` est émis
//! par `media.schedule_asset_purge()`. **La base les émet seule, et le même
//! déclencheur met aussi le traitement en file.**
//!
//! Un service zélé qui les émettrait à son tour produirait **deux traitements
//! par fichier** — et le doublon ne se verrait qu'en production. Leur absence de
//! ce fichier est donc **la décision**, pas un oubli : rien à déclarer, puisque
//! rien n'est émis par le code.
//!
//! # Le seul que le service émet
//!
//! La disparition **effective** d'un objet du stockage n'est annoncée par
//! personne : `schedule_asset_purge()` annonce l'INTENTION, jamais l'exécution.
//! Sans cette annonce, rien ne peut réagir à une perte définitive.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "media";
pub const AGGREGATE_ASSET: &str = "asset";

pub const ASSET_PURGED: &str = "media.asset.purged";

/// L'objet a réellement quitté le stockage. `rendition_bytes` voyage à part :
/// un objet et ses déclinaisons forment un bloc, et qui compte l'espace libéré
/// doit pouvoir faire la somme sans relire la base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPurged {
    pub bucket: String,
    pub object_key: String,
    pub byte_size: i64,
    pub rendition_bytes: i64,
    /// Nulle pour un objet appartenant à une personne : le quota est porté par
    /// l'organisation, et seule elle a de l'espace à récupérer.
    pub owner_organization_id: Option<Uuid>,
}
