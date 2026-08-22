//! **La mesure d'un flux au passage** : empreinte, poids, plafond.
//!
//! Le stockage consomme le flux ; le service, lui, doit connaître l'empreinte et
//! le poids **réels** à la fin. Un adaptateur les calcule en chemin, sans que le
//! fichier tienne jamais en mémoire : chaque tranche est hachée, comptée, puis
//! transmise telle quelle.
//!
//! # Le plafond coupe le flux plutôt que de le laisser finir
//!
//! `MEDIA_MAX_UPLOAD_BYTES` est un plafond **absolu**. Le laisser passer pour
//! refuser après coup reviendrait à écrire deux cents mégaoctets sur le disque
//! avant de les effacer — et à offrir à qui le veut un moyen de le remplir.

use actix_web::web::Bytes;
use futures_util::StreamExt;
use kernel::error::{ApiError, ErrorCode};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use crate::storage::{FluxOctets, StorageError, StorageResult};

/// L'état partagé entre l'adaptateur et l'appelant.
#[derive(Default)]
struct Etat {
    hacheur: Option<Sha256>,
    octets: u64,
    plafond_depasse: bool,
}

/// Ce qu'on tient du flux après l'avoir laissé passer.
#[derive(Clone)]
pub struct Mesure {
    etat: Arc<Mutex<Etat>>,
    plafond: u64,
}

impl Mesure {
    pub fn nouvelle(plafond: u64) -> Self {
        Self {
            etat: Arc::new(Mutex::new(Etat {
                hacheur: Some(Sha256::new()),
                octets: 0,
                plafond_depasse: false,
            })),
            plafond,
        }
    }

    /// Enveloppe le flux. Les tranches ressortent **inchangées**.
    pub fn envelopper(&self, flux: FluxOctets) -> FluxOctets {
        let etat = self.etat.clone();
        let plafond = self.plafond;

        Box::pin(flux.map(move |tranche| {
            let tranche = tranche?;
            let mut etat = etat.lock().expect("mesure du flux");

            etat.octets += tranche.len() as u64;
            if etat.octets > plafond {
                etat.plafond_depasse = true;
                return Err(StorageError::Rejected {
                    statut: 413,
                    corps: format!(
                        "plafond de dépôt dépassé : {} octets reçus, {plafond} autorisés",
                        etat.octets
                    ),
                });
            }
            if let Some(hacheur) = etat.hacheur.as_mut() {
                hacheur.update(&tranche);
            }
            Ok::<Bytes, StorageError>(tranche)
        }))
    }

    /// L'empreinte hexadécimale et le nombre d'octets réellement passés.
    ///
    /// À n'appeler **qu'une fois**, après que le flux a été entièrement
    /// consommé : le hacheur y est consommé, et un second appel rendrait
    /// l'empreinte du vide.
    pub fn resultat(&self) -> (String, u64) {
        let mut etat = self.etat.lock().expect("mesure du flux");
        let empreinte = etat
            .hacheur
            .take()
            .map(|h| h.finalize().iter().map(|o| format!("{o:02x}")).collect())
            .unwrap_or_default();
        (empreinte, etat.octets)
    }

    /// L'erreur à rendre quand le dépôt a échoué.
    ///
    /// **Le plafond prime** : un flux coupé par la limite doit sortir en « ce
    /// fichier dépasse la taille acceptée », pas en « le stockage est
    /// indisponible » — le second enverrait chercher une panne là où il n'y en a
    /// pas.
    pub fn erreur_ou(&self, erreur: StorageError) -> ApiError {
        if self.etat.lock().expect("mesure du flux").plafond_depasse {
            return ApiError::new(ErrorCode::MediaTooLarge)
                .field("file")
                .detail(format!("plafond absolu de {} octets", self.plafond));
        }
        ApiError::from(erreur)
    }
}

/// Un flux qui n'a jamais d'octets — celui d'un dépôt sans fichier.
pub fn flux_vide() -> FluxOctets {
    Box::pin(futures_util::stream::empty::<StorageResult<Bytes>>())
}

/// Un flux d'un seul bloc — pour les tests, et pour les contenus déjà en
/// mémoire.
pub fn flux_de(octets: Vec<u8>) -> FluxOctets {
    Box::pin(futures_util::stream::once(async move {
        Ok::<Bytes, StorageError>(Bytes::from(octets))
    }))
}

/// Un flux découpé en tranches — pour éprouver qu'une lecture par morceaux
/// produit la même empreinte qu'un bloc entier.
pub fn flux_en_tranches(octets: Vec<u8>, taille: usize) -> FluxOctets {
    let tranches: Vec<Bytes> = octets
        .chunks(taille.max(1))
        .map(Bytes::copy_from_slice)
        .collect();
    Box::pin(futures_util::stream::iter(
        tranches.into_iter().map(Ok::<Bytes, StorageError>),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn consommer(mesure: &Mesure, flux: FluxOctets) -> Result<(), StorageError> {
        let mut flux = mesure.envelopper(flux);
        while let Some(tranche) = flux.next().await {
            tranche?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn lempreinte_ne_depend_pas_du_decoupage() {
        let contenu = b"Financer l'adaptation, COP31 Belem".to_vec();

        let entier = Mesure::nouvelle(u64::MAX);
        consommer(&entier, flux_de(contenu.clone())).await.unwrap();

        let decoupe = Mesure::nouvelle(u64::MAX);
        consommer(&decoupe, flux_en_tranches(contenu.clone(), 3))
            .await
            .unwrap();

        // `resultat()` consomme le hacheur : on le lit **une fois**, comme le
        // service le fait.
        let (empreinte_entiere, poids) = entier.resultat();
        assert_eq!((empreinte_entiere.clone(), poids), decoupe.resultat());
        assert_eq!(empreinte_entiere.len(), 64);
    }

    #[tokio::test]
    async fn le_poids_mesure_est_le_poids_reel() {
        let mesure = Mesure::nouvelle(u64::MAX);
        consommer(&mesure, flux_en_tranches(vec![0_u8; 5000], 512))
            .await
            .unwrap();
        assert_eq!(mesure.resultat().1, 5000);
    }

    /// Le flux est **coupé**, et non laissé finir : sans cela, qui le veut
    /// remplit le disque avant qu'on le refuse.
    #[tokio::test]
    async fn le_plafond_coupe_le_flux() {
        let mesure = Mesure::nouvelle(1000);
        let erreur = consommer(&mesure, flux_en_tranches(vec![0_u8; 5000], 512))
            .await
            .unwrap_err();

        assert!(matches!(erreur, StorageError::Rejected { statut: 413, .. }));
        assert_eq!(
            mesure.erreur_ou(erreur).code,
            kernel::error::ErrorCode::MediaTooLarge
        );
    }

    #[tokio::test]
    async fn un_flux_vide_ne_mesure_rien() {
        let mesure = Mesure::nouvelle(u64::MAX);
        consommer(&mesure, flux_vide()).await.unwrap();
        assert_eq!(mesure.resultat().1, 0);
    }
}
