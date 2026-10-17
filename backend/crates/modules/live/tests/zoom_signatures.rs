//! Signatures et secrets Zoom, sans base ni réseau. Les vecteurs attendus ont
//! été calculés hors de Rust (Python, `hmac` et `hashlib`).

use live::jobs::sync_meeting::{corps_de_creation, sans_pwd, sans_secrets, verifier_lhote};
use live::zoom::client::delai_retry_after;
use live::zoom::{jeton_du_lien, jwt, webhook, Coffre};
use serde_json::json;

#[test]
fn la_signature_du_sdk_suit_le_vecteur_connu() {
    let signe = jwt::signer("cle-sdk", "secret-sdk", "85746065432", 0, 1_760_000_000);
    assert_eq!(
        signe.signature,
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhcHBLZXkiOiJjbGUtc2RrIiwic2RrS2V5IjoiY2xlLXNkayIsIm1uIjoiODU3NDYwNjU0MzIiLCJyb2xlIjowLCJpYXQiOjE3NTk5OTk5NzAsImV4cCI6MTc2MDAwNzE3MCwidG9rZW5FeHAiOjE3NjAwMDcxNzAsInZpZGVvX3dlYnJ0Y19tb2RlIjoxfQ.gVa5_Zy49tpIVK6Qhqxpse4yYEY8NSZ4GgfOiKAD5nU"
    );
    // Zoom exige au moins trente minutes entre l'émission et l'expiration.
    assert_eq!(signe.expire_a, 1_760_000_000 - 30 + 7200);
}

#[test]
fn la_signature_dune_notification_se_verifie() {
    let corps = br#"{"event":"meeting.started"}"#;
    let attendue = "v0=62c4cbabfd0d60eb383bcb8c364b6b8f0d41da31fd820a3f5989717f3ca39f22";
    assert!(webhook::signature_valide(
        "jeton-zoom",
        "1760000000",
        corps,
        attendue,
        1_760_000_100
    ));
    // Mauvais secret, corps altéré, horodatage trop ancien ou illisible.
    assert!(!webhook::signature_valide(
        "autre",
        "1760000000",
        corps,
        attendue,
        1_760_000_100
    ));
    assert!(!webhook::signature_valide(
        "jeton-zoom",
        "1760000000",
        br#"{"event":"meeting.ended"}"#,
        attendue,
        1_760_000_100
    ));
    assert!(!webhook::signature_valide(
        "jeton-zoom",
        "1760000000",
        corps,
        attendue,
        1_760_000_000 + 301
    ));
    assert!(!webhook::signature_valide(
        "jeton-zoom",
        "hier",
        corps,
        attendue,
        1_760_000_000
    ));
}

#[test]
fn la_validation_dadresse_rend_le_jeton_chiffre() {
    let reponse = webhook::reponse_de_validation("jeton-zoom", "qgg8vlvZRS6UYooatFL8Aw");
    assert_eq!(
        reponse,
        json!({
            "plainToken": "qgg8vlvZRS6UYooatFL8Aw",
            "encryptedToken": "4f81b9693bf3e806e150ffa2dff59e630b9b9439464171c8dc6b04650fc01275",
        })
    );
}

#[test]
fn le_jeton_tk_se_lit_dans_le_lien_personnel() {
    assert_eq!(
        jeton_du_lien("https://us06web.zoom.us/w/85746065432?tk=abc.DEF-ghi_1&pwd=xyz").as_deref(),
        Some("abc.DEF-ghi_1")
    );
    assert_eq!(
        jeton_du_lien("https://zoom.us/w/1?pwd=x&tk=a%2Eb").as_deref(),
        Some("a.b")
    );
    assert_eq!(jeton_du_lien("https://zoom.us/j/1?pwd=x"), None);
    assert_eq!(jeton_du_lien("https://zoom.us/j/1"), None);
    assert_eq!(jeton_du_lien("https://zoom.us/j/1?tk="), None);
}

#[test]
fn le_code_dacces_fait_laller_retour_chiffre() {
    // 32 octets en base64, comme `openssl rand -base64 32`.
    let coffre = Coffre::depuis_base64("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=").unwrap();
    let chiffre = coffre.chiffrer("Pavillon2026").unwrap();
    assert!(!chiffre.windows(12).any(|w| w == b"Pavillon2026"));
    assert_eq!(coffre.dechiffrer(&chiffre).unwrap(), "Pavillon2026");
    // Deux chiffrements du même code diffèrent (nonce aléatoire).
    assert_ne!(chiffre, coffre.chiffrer("Pavillon2026").unwrap());
    // Une autre clé ne lit rien.
    let autre = Coffre::depuis_base64("ZmVkY2JhOTg3NjU0MzIxMGZlZGNiYTk4NzY1NDMyMTA=").unwrap();
    assert!(autre.dechiffrer(&chiffre).is_err());
    assert!(Coffre::depuis_base64("Y291cnQ=").is_err());
}

#[test]
fn aucun_secret_ne_reste_dans_la_reponse_conservee() {
    let reponse = json!({
        "id": 85746065432u64,
        "password": "Pavillon2026",
        "encrypted_password": "chiffre",
        "h323_password": "1",
        "pstn_password": "2",
        "start_url": "https://zoom.us/s/1?zak=secret",
        "join_url": "https://zoom.us/j/1",
    });
    let gardee = sans_secrets(&reponse);
    for cle in [
        "password",
        "encrypted_password",
        "h323_password",
        "pstn_password",
        "start_url",
    ] {
        assert!(gardee.get(cle).is_none(), "{cle} conservé");
    }
    assert_eq!(gardee["join_url"], "https://zoom.us/j/1");
}

#[test]
fn la_reunion_est_creee_avec_inscription_et_approbation_automatique() {
    let debut = time::macros::datetime!(2026-11-10 11:00 -3);
    let corps = corps_de_creation("Atelier", None, Some(debut), Some(90), "America/Belem");
    assert_eq!(corps["type"], 2);
    assert_eq!(corps["start_time"], "2026-11-10T14:00:00Z");
    assert_eq!(corps["duration"], 90);
    assert_eq!(corps["settings"]["approval_type"], 0);
    assert_eq!(corps["settings"]["registration_type"], 1);
    assert_eq!(corps["settings"]["waiting_room"], false);
}

#[test]
fn le_lien_generique_perd_son_code() {
    assert_eq!(
        sans_pwd("https://zoom.us/j/1?pwd=abc"),
        "https://zoom.us/j/1"
    );
    assert_eq!(
        sans_pwd("https://zoom.us/j/1?a=1&pwd=abc&b=2"),
        "https://zoom.us/j/1?a=1&b=2"
    );
    assert_eq!(sans_pwd("https://zoom.us/j/1"), "https://zoom.us/j/1");
    let gardee = sans_secrets(&json!({ "join_url": "https://zoom.us/j/1?pwd=abc" }));
    assert_eq!(gardee["join_url"], "https://zoom.us/j/1");
}

#[test]
fn retry_after_en_secondes_ou_en_date() {
    let maintenant = time::macros::datetime!(2026-10-17 10:00 UTC);
    assert_eq!(delai_retry_after("120", maintenant), Some(120));
    assert_eq!(
        delai_retry_after("2026-10-17T10:05:00Z", maintenant),
        Some(300)
    );
    assert_eq!(delai_retry_after("999999", maintenant), Some(86_400));
    assert_eq!(delai_retry_after("bientôt", maintenant), None);
}

#[test]
fn une_reunion_dun_autre_hote_est_refusee() {
    let reunion = json!({ "host_email": "Pavillon@ifdd.org", "host_id": "abc" });
    assert!(verifier_lhote(&reunion, "me").is_ok());
    assert!(verifier_lhote(&reunion, "pavillon@ifdd.org").is_ok());
    assert!(verifier_lhote(&reunion, "abc").is_ok());
    assert!(verifier_lhote(&reunion, "autre@ifdd.org").is_err());
}
