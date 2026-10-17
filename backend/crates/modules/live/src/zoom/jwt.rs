//! La signature du Meeting SDK pour le web : un JWT HS256 signé avec le secret
//! de l'application « General ». Champs d'après
//! https://developers.zoom.us/docs/meeting-sdk/auth/ (relu le 17/10) ; `sdkKey`
//! doublonne `appKey` comme l'exemple officiel, pour les versions du SDK qui le
//! lisent encore.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

/// Deux heures : l'exigence de Zoom est d'au moins trente minutes.
pub const DUREE_SECONDES: i64 = 7200;
/// Marge contre une horloge de serveur en avance sur celle de Zoom.
const AVANCE_SECONDES: i64 = 30;

pub struct SignatureSdk {
    pub signature: String,
    pub expire_a: i64,
}

/// `role` 0 : participant. `maintenant` en secondes depuis l'époque Unix.
pub fn signer(
    sdk_key: &str,
    sdk_secret: &str,
    numero_reunion: &str,
    role: u8,
    maintenant: i64,
) -> SignatureSdk {
    let iat = maintenant - AVANCE_SECONDES;
    let exp = iat + DUREE_SECONDES;
    // Écrit à la main : l'ordre des clés de `serde_json` dépend des
    // fonctionnalités unifiées à la compilation, et le vecteur de test aussi.
    let entete = r#"{"alg":"HS256","typ":"JWT"}"#;
    let cle = Value::from(sdk_key).to_string();
    let charge = format!(
        r#"{{"appKey":{cle},"sdkKey":{cle},"mn":{},"role":{role},"iat":{iat},"exp":{exp},"tokenExp":{exp},"video_webrtc_mode":1}}"#,
        Value::from(numero_reunion)
    );
    let a_signer = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(entete),
        URL_SAFE_NO_PAD.encode(charge)
    );
    let mut mac =
        Hmac::<Sha256>::new_from_slice(sdk_secret.as_bytes()).expect("HMAC accepte toute clé");
    mac.update(a_signer.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    SignatureSdk {
        signature: format!("{a_signer}.{signature}"),
        expire_a: exp,
    }
}
