//! Le stockage sur fichiers — celui des tests, et du développement hors ligne.
//!
//! # La clé est un chemin, et c'est là qu'est le danger
//!
//! Une clé d'objet vient d'un nom de fichier déposé par un client. Écrite telle
//! quelle sous une racine, `../../etc/passwd` sortirait de la racine. La
//! normalisation de `domain::keys` l'interdit déjà — elle ne laisse passer ni
//! barre oblique en tête, ni point isolé —, mais **ce fichier ne s'y fie pas** :
//! il vérifie lui-même que le chemin résolu reste sous la racine. Une garde qui
//! dépend d'une autre garde n'en est pas une.

use async_trait::async_trait;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

use super::{FluxOctets, ObjectInfo, ObjectStore, StorageError, StorageResult};

pub struct FilesystemStore {
    racine: PathBuf,
}

impl FilesystemStore {
    pub fn new(racine: &str) -> Self {
        Self {
            racine: PathBuf::from(racine),
        }
    }

    /// Le chemin d'une clé, **borné à la racine**.
    fn chemin(&self, key: &str) -> StorageResult<PathBuf> {
        if key.is_empty() {
            return Err(StorageError::Rejected {
                statut: 400,
                corps: "clé vide".to_owned(),
            });
        }
        let mut chemin = self.racine.clone();
        for segment in key.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(StorageError::Rejected {
                    statut: 400,
                    corps: format!("clé refusée : « {key} » sort de la racine"),
                });
            }
            chemin.push(segment);
        }
        Ok(chemin)
    }

    async fn creer_le_dossier(chemin: &Path) -> StorageResult<()> {
        if let Some(parent) = chemin.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Unavailable(e.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl ObjectStore for FilesystemStore {
    /// Écrit tranche par tranche : la mémoire employée est celle d'une tranche,
    /// quelle que soit la taille du fichier.
    async fn put_stream(
        &self,
        key: &str,
        _mime_type: &str,
        mut contenu: FluxOctets,
    ) -> StorageResult<u64> {
        let chemin = self.chemin(key)?;
        Self::creer_le_dossier(&chemin).await?;

        let mut fichier = tokio::fs::File::create(&chemin)
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))?;

        let mut ecrits = 0_u64;
        while let Some(tranche) = contenu.next().await {
            let tranche = tranche?;
            fichier
                .write_all(&tranche)
                .await
                .map_err(|e| StorageError::Unavailable(e.to_string()))?;
            ecrits += tranche.len() as u64;
        }
        fichier
            .flush()
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))?;

        Ok(ecrits)
    }

    async fn put(&self, key: &str, _mime_type: &str, contenu: Vec<u8>) -> StorageResult<()> {
        let chemin = self.chemin(key)?;
        Self::creer_le_dossier(&chemin).await?;
        tokio::fs::write(&chemin, contenu)
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))
    }

    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let chemin = self.chemin(key)?;
        tokio::fs::read(&chemin).await.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StorageError::NotFound(key.to_owned()),
            _ => StorageError::Unavailable(e.to_string()),
        })
    }

    async fn head(&self, key: &str) -> StorageResult<ObjectInfo> {
        let chemin = self.chemin(key)?;
        let meta = tokio::fs::metadata(&chemin)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound(key.to_owned()),
                _ => StorageError::Unavailable(e.to_string()),
            })?;
        Ok(ObjectInfo {
            byte_size: meta.len() as i64,
        })
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let chemin = self.chemin(key)?;
        match tokio::fs::remove_file(&chemin).await {
            Ok(()) => Ok(()),
            // Supprimer ce qui n'existe pas est un succès : la purge se rejoue.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError::Unavailable(e.to_string())),
        }
    }

    async fn rename(&self, de: &str, vers: &str) -> StorageResult<()> {
        let source = self.chemin(de)?;
        let cible = self.chemin(vers)?;
        Self::creer_le_dossier(&cible).await?;
        tokio::fs::rename(&source, &cible)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound(de.to_owned()),
                _ => StorageError::Unavailable(e.to_string()),
            })
    }

    fn engine(&self) -> &'static str {
        "filesystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_cle_qui_remonte_est_refusee() {
        let store = FilesystemStore::new("/tmp/epavillon-test");
        for cle in ["../secret", "a/../../b", "./x", "a//b", ""] {
            assert!(
                store.chemin(cle).is_err(),
                "« {cle} » aurait dû être refusée"
            );
        }
    }

    #[test]
    fn une_cle_de_la_convention_passe() {
        let store = FilesystemStore::new("/tmp/epavillon-test");
        let chemin = store.chemin("2026/08/abc/logo.png").unwrap();
        assert!(chemin.ends_with("2026/08/abc/logo.png"));
        assert!(chemin.starts_with("/tmp/epavillon-test"));
    }
}
