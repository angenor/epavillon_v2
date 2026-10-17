//! AES-256-GCM, clé `LIVE_SECRETS_KEY` : le code d'accès et le lien hôte d'une
//! réunion ne sont jamais écrits en clair (080_live.sql, D3). Le nonce de douze
//! octets précède le texte chiffré dans la colonne.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use kernel::error::{ApiError, Result};
use rand_core::{OsRng, RngCore};

const NONCE: usize = 12;

pub struct Coffre {
    chiffreur: Aes256Gcm,
}

impl Coffre {
    pub fn depuis_base64(cle: &str) -> Result<Self> {
        let octets = STANDARD
            .decode(cle.trim())
            .map_err(|_| ApiError::internal("LIVE_SECRETS_KEY illisible"))?;
        let chiffreur = Aes256Gcm::new_from_slice(&octets)
            .map_err(|_| ApiError::internal("LIVE_SECRETS_KEY doit coder 32 octets"))?;
        Ok(Self { chiffreur })
    }

    pub fn chiffrer(&self, clair: &str) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE];
        OsRng.fill_bytes(&mut nonce);
        let chiffre = self
            .chiffreur
            .encrypt(Nonce::from_slice(&nonce), clair.as_bytes())
            .map_err(|_| ApiError::internal("chiffrement d'un secret de réunion impossible"))?;
        let mut sortie = nonce.to_vec();
        sortie.extend(chiffre);
        Ok(sortie)
    }

    pub fn dechiffrer(&self, octets: &[u8]) -> Result<String> {
        if octets.len() <= NONCE {
            return Err(ApiError::internal("secret de réunion tronqué"));
        }
        let (nonce, chiffre) = octets.split_at(NONCE);
        let clair = self
            .chiffreur
            .decrypt(Nonce::from_slice(nonce), chiffre)
            .map_err(|_| {
                ApiError::internal("secret de réunion indéchiffrable : LIVE_SECRETS_KEY a changé ?")
            })?;
        String::from_utf8(clair).map_err(|_| ApiError::internal("secret de réunion non UTF-8"))
    }
}
