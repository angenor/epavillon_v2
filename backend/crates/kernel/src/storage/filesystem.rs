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

    async fn get_range(&self, key: &str, debut: u64, fin: u64) -> StorageResult<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let chemin = self.chemin(key)?;
        let introuvable = |e: std::io::Error| match e.kind() {
            std::io::ErrorKind::NotFound => StorageError::NotFound(key.to_owned()),
            _ => StorageError::Unavailable(e.to_string()),
        };
        let mut fichier = tokio::fs::File::open(&chemin).await.map_err(introuvable)?;
        let taille = fichier.metadata().await.map_err(introuvable)?.len();
        if debut >= taille || fin < debut {
            return Err(StorageError::Rejected {
                statut: 416,
                corps: format!("plage {debut}-{fin} hors d'un objet de {taille} octets"),
            });
        }
        let fin = fin.min(taille - 1);
        fichier
            .seek(std::io::SeekFrom::Start(debut))
            .await
            .map_err(introuvable)?;
        let mut octets = vec![0_u8; (fin - debut + 1) as usize];
        fichier.read_exact(&mut octets).await.map_err(introuvable)?;
        Ok(octets)
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

    struct Dossier(PathBuf);

    impl Drop for Dossier {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn stockage_de_dix_octets() -> (FilesystemStore, Dossier) {
        let dossier =
            std::env::temp_dir().join(format!("epavillon-plage-{}", uuid::Uuid::new_v4().simple()));
        let store = FilesystemStore::new(&dossier.to_string_lossy());
        store
            .put("doc/guide.pdf", "application/pdf", b"0123456789".to_vec())
            .await
            .expect("dépôt");
        (store, Dossier(dossier))
    }

    #[tokio::test]
    async fn une_plage_rend_ses_octets_bornes_comprises() {
        let (store, _dossier) = stockage_de_dix_octets().await;
        let lire = |a, b| store.get_range("doc/guide.pdf", a, b);
        assert_eq!(lire(0, 3).await.unwrap(), b"0123");
        assert_eq!(lire(6, 9).await.unwrap(), b"6789");
        assert_eq!(lire(9, 9).await.unwrap(), b"9");
        assert_eq!(lire(4, 4).await.unwrap(), b"4");
    }

    #[tokio::test]
    async fn une_fin_au_dela_s_arrete_au_dernier_octet() {
        let (store, _dossier) = stockage_de_dix_octets().await;
        assert_eq!(
            store.get_range("doc/guide.pdf", 7, 500).await.unwrap(),
            b"789"
        );
    }

    #[tokio::test]
    async fn un_debut_au_dela_est_refuse() {
        let (store, _dossier) = stockage_de_dix_octets().await;
        assert!(matches!(
            store.get_range("doc/guide.pdf", 10, 12).await,
            Err(StorageError::Rejected { statut: 416, .. })
        ));
        assert!(matches!(
            store.get_range("doc/absent.pdf", 0, 1).await,
            Err(StorageError::NotFound(_))
        ));
    }

    #[test]
    fn une_cle_de_la_convention_passe() {
        let store = FilesystemStore::new("/tmp/epavillon-test");
        let chemin = store.chemin("2026/08/abc/logo.png").unwrap();
        assert!(chemin.ends_with("2026/08/abc/logo.png"));
        assert!(chemin.starts_with("/tmp/epavillon-test"));
    }
}
