//! Argon2id, jetons aléatoires et empreintes.
//!
//! `COMMENT ON COLUMN identity.accounts.password_hash` le dit : « aucune
//! fonction SQL ne doit vérifier de mot de passe ». Le calcul et la
//! comparaison vivent entièrement ici.

use argon2::password_hash::SaltString;
use argon2::password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::error::{ApiError, Result};

/// Paramètres OWASP pour Argon2id : 19 MiB, 2 itérations, parallélisme 1.
/// Ils placent le coût entre 50 et 100 ms — au-delà la connexion devient
/// lente, en deçà le hachage cesse de protéger.
const MEMOIRE_KIB: u32 = 19_456;
const ITERATIONS: u32 = 2;
const PARALLELISME: u32 = 1;

pub struct Passwords {
    argon: Argon2<'static>,
    /// Empreinte d'un mot de passe qui n'existe pas, calculée une fois au
    /// démarrage. Vérifier contre elle quand l'adresse est inconnue est la
    /// seule façon de rendre le même temps de réponse (FR-020, SC-001) : un
    /// message identique ne suffit pas, le temps parle aussi.
    empreinte_factice: String,
}

impl Passwords {
    pub fn new() -> Result<Self> {
        let params = Params::new(MEMOIRE_KIB, ITERATIONS, PARALLELISME, None)
            .map_err(|e| ApiError::internal(format!("paramètres Argon2id invalides : {e}")))?;
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let sel = SaltString::generate(&mut OsRng);
        let empreinte_factice = argon
            .hash_password(b"empreinte-factice-jamais-utilisee", &sel)
            .map_err(|e| ApiError::internal(format!("calcul de l'empreinte factice : {e}")))?
            .to_string();

        Ok(Self {
            argon,
            empreinte_factice,
        })
    }

    pub fn hash(&self, mot_de_passe: &str) -> Result<String> {
        let sel = SaltString::generate(&mut OsRng);
        self.argon
            .hash_password(mot_de_passe.as_bytes(), &sel)
            .map(|h| h.to_string())
            .map_err(|e| ApiError::internal(format!("hachage du mot de passe : {e}")))
    }

    pub fn verify(&self, mot_de_passe: &str, empreinte_phc: &str) -> bool {
        let Ok(attendue) = PasswordHash::new(empreinte_phc) else {
            return false;
        };
        self.argon
            .verify_password(mot_de_passe.as_bytes(), &attendue)
            .is_ok()
    }

    /// Consomme le même temps qu'une vérification réelle, et jette le résultat.
    /// Appelée quand l'adresse est inconnue ou sans compte mot de passe.
    pub fn burn(&self, mot_de_passe: &str) {
        let _ = self.verify(mot_de_passe, &self.empreinte_factice);
    }
}

/// Jeton opaque de 32 octets, rendu en base64url sans remplissage.
/// Le clair ne vit que dans le cookie ou dans le courriel ; la base n'en garde
/// que l'empreinte.
pub fn random_token() -> String {
    let mut octets = [0u8; 32];
    OsRng.fill_bytes(&mut octets);
    URL_SAFE_NO_PAD.encode(octets)
}

pub fn token_hash(jeton: &str) -> [u8; 32] {
    let mut hacheur = Sha256::new();
    hacheur.update(jeton.as_bytes());
    hacheur.finalize().into()
}

/// Comparaison à temps constant : sert au secret partagé du relais de courriel.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
