//! Le contrat de stockage — **un trait, deux implémentations**.
//!
//! Exactement le patron de `kernel::mail` : la configuration choisit, le service
//! ne sait pas par où passent les octets.
//!
//! # Pourquoi deux implémentations, et pourquoi les tests prennent la seconde
//!
//! `make check-db` exécute `down -v`, ce qui **efface le layout de Garage** — le
//! Makefile le rappelle lui-même. Des tests d'intégration qui frapperaient S3
//! échoueraient après chaque vérification complète, et l'on prendrait
//! l'habitude de les sauter. Or *« une commande de vérification qui échoue
//! toujours de la même façon finit par se lire comme du bruit »* est déjà écrit
//! dans les pièges du dépôt.
//!
//! Les tests d'intégration tournent donc sur le **système de fichiers** : ils
//! exercent le service entier, tout le SQL, la déduplication, les quotas, le
//! rattachement et la fabrication des déclinaisons. Le stockage S3 réel se
//! vérifie **à la main**, par le point de contrôle du quickstart — exactement
//! comme B1 a vérifié la chaîne de courriel dans Mailpit (B6, R7).
//!
//! # Cinq verbes, et `rename` en est un
//!
//! Le dépôt écrit d'abord sur une clé temporaire, calcule l'empreinte au fil du
//! flux, puis **renomme** vers la clé définitive ou **supprime** si le contenu
//! est déjà connu (R10). Sans `rename`, il faudrait relire et réécrire les deux
//! cents mégaoctets d'un fond vidéo pour le déplacer.

use actix_web::web::Bytes;
use async_trait::async_trait;
use futures_util::Stream;
use kernel::error::{ApiError, ErrorCode};
use std::pin::Pin;
use std::sync::Arc;

pub mod filesystem;
pub mod s3;
pub mod sigv4;

/// Ce qu'un `HEAD` rend d'un objet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectInfo {
    pub byte_size: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("objet absent du stockage : {0}")]
    NotFound(String),
    #[error("stockage injoignable : {0}")]
    Unavailable(String),
    #[error("stockage : réponse {statut} — {corps}")]
    Rejected { statut: u16, corps: String },
}

impl From<StorageError> for ApiError {
    /// **Aucune description n'est écrite** quand un défaut de stockage remonte
    /// vers un client : le détail — adresse du nœud, clé d'accès dans un
    /// en-tête refusé — part dans la trace, jamais dans la réponse.
    fn from(erreur: StorageError) -> Self {
        match erreur {
            StorageError::NotFound(ref cle) => {
                ApiError::not_found().detail(format!("objet absent du stockage : {cle} ({erreur})"))
            }
            autre => ApiError::new(ErrorCode::MediaStorageUnavailable).detail(autre),
        }
    }
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Un flux d'octets à déposer. C'est ce qui permet à un fond vidéo de deux
/// cents mégaoctets de traverser sans jamais tenir en mémoire.
pub type FluxOctets = Pin<Box<dyn Stream<Item = StorageResult<Bytes>> + Send>>;

#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Dépose un objet **en flux**. Rend le nombre d'octets réellement écrits —
    /// c'est lui que le service compare au poids annoncé, jamais celui que le
    /// client a déclaré.
    ///
    /// `mime_type` voyage parce que S3 le porte en en-tête et que le stockage le
    /// rend tel quel au navigateur.
    async fn put_stream(
        &self,
        key: &str,
        mime_type: &str,
        contenu: FluxOctets,
    ) -> StorageResult<u64>;

    /// Dépose un contenu **déjà en mémoire**. Réservé à ce qui est petit par
    /// construction : une déclinaison d'image pèse quelques centaines de
    /// kilo-octets, et la fabriquer exige de toute façon de la tenir entière.
    async fn put(&self, key: &str, mime_type: &str, contenu: Vec<u8>) -> StorageResult<()>;

    async fn get(&self, key: &str) -> StorageResult<Vec<u8>>;

    async fn head(&self, key: &str) -> StorageResult<ObjectInfo>;

    /// Idempotente : supprimer ce qui n'existe pas est un succès. La purge se
    /// rejoue, et faire échouer un second passage laisserait le travail mourir
    /// sur un objet déjà parti.
    async fn delete(&self, key: &str) -> StorageResult<()>;

    /// Déplace un objet. S3 n'a pas de verbe de déplacement : l'implémentation
    /// copie puis supprime, ce que le protocole fait **côté serveur**, sans que
    /// les octets traversent le réseau.
    async fn rename(&self, de: &str, vers: &str) -> StorageResult<()>;

    /// Le nom du moteur, pour la trace. Sans lui, un test rouge ne dit pas
    /// contre quoi il a échoué.
    fn engine(&self) -> &'static str;
}

/// Construit le stockage choisi par la configuration.
pub fn build(cfg: &kernel::config::MediaConfig) -> Arc<dyn ObjectStore> {
    match cfg.storage {
        kernel::config::MediaStorage::Filesystem => {
            Arc::new(filesystem::FilesystemStore::new(&cfg.fs_root))
        }
        kernel::config::MediaStorage::S3 => Arc::new(s3::S3Store::new(&cfg.s3)),
    }
}
