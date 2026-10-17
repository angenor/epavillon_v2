//! Les notifications de Zoom : validation de l'adresse et signature
//! `x-zm-signature`, toutes deux en HMAC-SHA256 du jeton secret du webhook.

use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

/// Au-delà, une notification signée est traitée comme un rejeu.
pub const FRAICHEUR_SECONDES: i64 = 300;

pub fn hmac_hex(secret: &str, message: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepte toute clé");
    mac.update(message);
    hex(&mac.finalize().into_bytes())
}

pub fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

/// Réponse à `endpoint.url_validation`.
pub fn reponse_de_validation(secret: &str, jeton_clair: &str) -> Value {
    json!({
        "plainToken": jeton_clair,
        "encryptedToken": hmac_hex(secret, jeton_clair.as_bytes()),
    })
}

/// `v0=` + hex(HMAC(secret, "v0:{horodatage}:{corps}")), comparé à temps
/// constant, horodatage en secondes et frais de moins de cinq minutes.
pub fn signature_valide(
    secret: &str,
    horodatage: &str,
    corps: &[u8],
    signature: &str,
    maintenant: i64,
) -> bool {
    let Ok(instant) = horodatage.trim().parse::<i64>() else {
        return false;
    };
    if (maintenant - instant).abs() > FRAICHEUR_SECONDES {
        return false;
    }
    let mut message = format!("v0:{}:", horodatage.trim()).into_bytes();
    message.extend_from_slice(corps);
    let attendue = format!("v0={}", hmac_hex(secret, &message));
    kernel::crypto::constant_time_eq(&attendue, signature.trim())
}
