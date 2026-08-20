//! Jeton d'accès : un **renvoi signé vers une session**, pas un porteur de
//! droits.
//!
//! `identity.sessions` ne porte qu'une empreinte de jeton de *rafraîchissement*
//! — il n'existe aucune colonne où loger un jeton d'accès opaque. C'est le
//! modèle qui a décidé de la forme : ce qui n'est pas stocké doit être
//! auto-porteur, donc signé (research.md § R1).
//!
//! **Aucune permission n'y figure.** Un jeton portant ses droits les figerait un
//! quart d'heure ; comme toute route autorisée interroge de toute façon
//! `identity.has_permission()` avec sa portée, la lecture en base ne coûte rien
//! de plus — et une suspension coupe les sessions à la requête suivante (FR-033).
//!
//! La forme est celle d'un JWT `EdDSA`. Elle est composée ici plutôt que par une
//! bibliothèque parce que le format de clé décide de l'ergonomie : `.env` tient
//! une ligne, et une clé Ed25519 y entre comme **32 octets en base64** — pas
//! comme un bloc PEM à replier. Signature et vérification, elles, ne sont pas
//! réécrites : elles viennent d'`ed25519-dalek`.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use kernel::config::Secret;
use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{PersonId, SessionId};

const ALGORITHME: &str = "EdDSA";

#[derive(Serialize, Deserialize)]
struct Entete {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Serialize, Deserialize)]
struct Charge {
    sub: Uuid,
    sid: Uuid,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessClaims {
    pub person_id: PersonId,
    pub session_id: SessionId,
    pub expires_at: OffsetDateTime,
}

pub struct AccessTokenCodec {
    signature: SigningKey,
    verification: VerifyingKey,
    /// Identifiant de clé, dérivé de la clé publique. Une seule clé vit dans ce
    /// jalon, mais le format en accepte plusieurs : la rotation reste possible
    /// plus tard sans changer le contrat.
    kid: String,
    duree: Duration,
}

impl AccessTokenCodec {
    pub fn new(cle: &Secret, duree: Duration) -> Result<Self> {
        let graine = decoder_graine(cle.expose()).ok_or_else(|| {
            ApiError::internal(
                "AUTH_SIGNING_KEY doit porter 32 octets en base64 ou 64 signes hexadécimaux.",
            )
        })?;

        let signature = SigningKey::from_bytes(&graine);
        let verification = signature.verifying_key();
        let kid = empreinte_courte(verification.as_bytes());

        Ok(Self {
            signature,
            verification,
            kid,
            duree,
        })
    }

    pub fn duree(&self) -> Duration {
        self.duree
    }

    pub fn issue(&self, person_id: PersonId, session_id: SessionId) -> Result<String> {
        let maintenant = OffsetDateTime::now_utc();
        let expiration = maintenant + time::Duration::seconds(self.duree.as_secs() as i64);

        let entete = Entete {
            alg: ALGORITHME.to_owned(),
            typ: "JWT".to_owned(),
            kid: self.kid.clone(),
        };
        let charge = Charge {
            sub: person_id.as_uuid(),
            sid: session_id.as_uuid(),
            iat: maintenant.unix_timestamp(),
            exp: expiration.unix_timestamp(),
        };

        let corps = format!("{}.{}", encoder_json(&entete)?, encoder_json(&charge)?);
        let signature = self.signature.sign(corps.as_bytes());

        Ok(format!(
            "{corps}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    /// Rend `None` pour tout jeton qui ne vaut pas : mal formé, signé d'une
    /// autre clé, altéré, périmé. Aucune de ces distinctions n'a de destinataire
    /// — l'appelant n'a qu'une décision à prendre, et pas de message à composer.
    pub fn verify(&self, jeton: &str) -> Option<AccessClaims> {
        let mut morceaux = jeton.split('.');
        let entete_b64 = morceaux.next()?;
        let charge_b64 = morceaux.next()?;
        let signature_b64 = morceaux.next()?;
        if morceaux.next().is_some() {
            return None;
        }

        let entete: Entete = decoder_json(entete_b64)?;
        if entete.alg != ALGORITHME || entete.kid != self.kid {
            return None;
        }

        let octets = URL_SAFE_NO_PAD.decode(signature_b64).ok()?;
        let signature = Signature::from_slice(&octets).ok()?;
        let corps = format!("{entete_b64}.{charge_b64}");
        self.verification
            .verify(corps.as_bytes(), &signature)
            .ok()?;

        let charge: Charge = decoder_json(charge_b64)?;
        let expiration = OffsetDateTime::from_unix_timestamp(charge.exp).ok()?;
        if expiration <= OffsetDateTime::now_utc() {
            return None;
        }

        Some(AccessClaims {
            person_id: PersonId(charge.sub),
            session_id: SessionId(charge.sid),
            expires_at: expiration,
        })
    }
}

fn encoder_json<T: Serialize>(valeur: &T) -> Result<String> {
    serde_json::to_vec(valeur)
        .map(|octets| URL_SAFE_NO_PAD.encode(octets))
        .map_err(ApiError::internal)
}

fn decoder_json<T: for<'de> Deserialize<'de>>(segment: &str) -> Option<T> {
    let octets = URL_SAFE_NO_PAD.decode(segment).ok()?;
    serde_json::from_slice(&octets).ok()
}

/// Trois écritures acceptées pour la même chose : base64 sans remplissage — ce
/// que rend le générateur de jetons du noyau —, base64 standard — ce que rend
/// `openssl rand -base64 32` — et hexadécimal.
fn decoder_graine(valeur: &str) -> Option<[u8; 32]> {
    let valeur = valeur.trim();

    // C'est la LONGUEUR obtenue qui départage, pas l'ordre des essais : soixante
    // -quatre signes hexadécimaux forment aussi du base64 valide, et le premier
    // décodage qui aboutit n'est donc pas forcément le bon.
    [
        decoder_hexa(valeur),
        URL_SAFE_NO_PAD.decode(valeur).ok(),
        STANDARD.decode(valeur).ok(),
    ]
    .into_iter()
    .flatten()
    .find(|octets| octets.len() == 32)?
    .try_into()
    .ok()
}

fn decoder_hexa(valeur: &str) -> Option<Vec<u8>> {
    if valeur.len() != 64 || !valeur.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&valeur[i..i + 2], 16).ok())
        .collect()
}

fn empreinte_courte(octets: &[u8]) -> String {
    let empreinte = Sha256::digest(octets);
    empreinte[..4].iter().map(|o| format!("{o:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codec(duree: Duration) -> AccessTokenCodec {
        // Graine fixe : le test doit pouvoir rejouer la même signature.
        let cle = URL_SAFE_NO_PAD.encode([7u8; 32]);
        AccessTokenCodec::new(&Secret::from(cle), duree).expect("codec")
    }

    #[test]
    fn un_jeton_emis_se_verifie() {
        let codec = codec(Duration::from_secs(900));
        let personne = PersonId(Uuid::now_v7());
        let session = SessionId(Uuid::now_v7());

        let jeton = codec.issue(personne, session).expect("émission");
        let lu = codec.verify(&jeton).expect("vérification");

        assert_eq!(lu.person_id, personne);
        assert_eq!(lu.session_id, session);
    }

    #[test]
    fn un_jeton_altere_est_refuse() {
        let codec = codec(Duration::from_secs(900));
        let jeton = codec
            .issue(PersonId(Uuid::now_v7()), SessionId(Uuid::now_v7()))
            .expect("émission");

        let mut morceaux: Vec<&str> = jeton.split('.').collect();
        let autre = codec
            .issue(PersonId(Uuid::now_v7()), SessionId(Uuid::now_v7()))
            .expect("émission");
        let charge_etrangere: Vec<&str> = autre.split('.').collect();
        morceaux[1] = charge_etrangere[1];

        assert!(codec.verify(&morceaux.join(".")).is_none());
    }

    #[test]
    fn un_jeton_perime_est_refuse() {
        let codec = codec(Duration::from_secs(0));
        let jeton = codec
            .issue(PersonId(Uuid::now_v7()), SessionId(Uuid::now_v7()))
            .expect("émission");

        assert!(codec.verify(&jeton).is_none());
    }

    /// Le `.env` de développement porte la clé en hexadécimal : soixante-quatre
    /// signes, qui forment aussi du base64 valide de quarante-huit octets. Le
    /// décodage doit trancher par la longueur, pas par l'ordre des essais.
    #[test]
    fn la_cle_se_lit_en_hexadecimal_comme_en_base64() {
        let hexa = "7e9a5b2679cacd529920389e4c214730f9eb4d484e3208cd5a328da845b086a4";
        let codec = AccessTokenCodec::new(&Secret::from(hexa.to_owned()), Duration::from_secs(900))
            .expect("clé hexadécimale");

        let jeton = codec
            .issue(PersonId(Uuid::now_v7()), SessionId(Uuid::now_v7()))
            .expect("émission");
        assert!(codec.verify(&jeton).is_some());
    }

    #[test]
    fn une_cle_de_mauvaise_longueur_est_refusee() {
        assert!(AccessTokenCodec::new(
            &Secret::from("trop-courte".to_owned()),
            Duration::from_secs(900)
        )
        .is_err());
    }

    #[test]
    fn une_autre_cle_ne_verifie_rien() {
        let jeton = codec(Duration::from_secs(900))
            .issue(PersonId(Uuid::now_v7()), SessionId(Uuid::now_v7()))
            .expect("émission");

        let autre = AccessTokenCodec::new(
            &Secret::from(URL_SAFE_NO_PAD.encode([9u8; 32])),
            Duration::from_secs(900),
        )
        .expect("codec");

        assert!(autre.verify(&jeton).is_none());
    }
}
