//! Le client Zoom contre un petit serveur HTTP local qui imite l'API : jeton
//! gardé, création, inscription retrouvée dans la liste, erreurs passagères.

mod commun;

use commun::http::serveur;
use kernel::config::{Secret, ZoomConfig};
use live::zoom::{ZoomClient, ZoomError};
use serde_json::json;

fn client(adresse: &str) -> ZoomClient {
    ZoomClient::new(ZoomConfig {
        account_id: "compte".into(),
        client_id: "client".into(),
        client_secret: Secret::from("secret".to_owned()),
        host_user: "me".into(),
        sdk_key: "sdk".into(),
        sdk_secret: Secret::from("sdk-secret".to_owned()),
        webhook_secret_token: None,
        secrets_key: Secret::from("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=".to_owned()),
        api_base_url: format!("{adresse}/v2"),
        oauth_url: format!("{adresse}/oauth/token"),
    })
    .unwrap()
}

fn api(methode: &str, chemin: &str) -> (u16, String) {
    let jeton = json!({ "access_token": "jeton", "expires_in": 3600 }).to_string();
    match (methode, chemin.split('?').next().unwrap_or_default()) {
        ("POST", "/oauth/token") => (200, jeton),
        ("POST", "/v2/users/me/meetings") => (
            201,
            json!({ "id": 85746065432u64, "password": "abc", "join_url": "https://zoom.us/j/85746065432" })
                .to_string(),
        ),
        // Adresse déjà inscrite : refus, l'inscrit se retrouve dans la liste.
        ("POST", "/v2/meetings/85746065432/registrants") => (
            400,
            json!({ "code": 3027, "message": "already registered" }).to_string(),
        ),
        ("GET", "/v2/meetings/85746065432/registrants") if chemin.contains("status=approved") => (
            200,
            json!({
                "registrants": [
                    { "id": "autre", "email": "x@exemple.org", "join_url": "https://zoom.us/w/1?tk=x" },
                    { "id": "reg-1", "email": "Awa.Diallo@Exemple.org", "join_url": "https://zoom.us/w/1?tk=tk-1" },
                ],
                "next_page_token": "",
            })
            .to_string(),
        ),
        ("POST", "/v2/meetings/1/registrants") => (429, "{}".to_owned()),
        _ => (404, json!({ "code": 3001, "message": "introuvable" }).to_string()),
    }
}

#[tokio::test]
async fn la_creation_garde_le_jeton_dacces() {
    let (adresse, journal) = serveur(std::sync::Arc::new(api)).await;
    let zoom = client(&adresse);

    let reunion = zoom
        .creer_reunion(&json!({ "topic": "Atelier" }))
        .await
        .unwrap();
    assert_eq!(reunion["id"], 85746065432u64);
    zoom.creer_reunion(&json!({ "topic": "Atelier" }))
        .await
        .unwrap();

    let journal = journal.lock().unwrap();
    let jetons = journal
        .iter()
        .filter(|l| l.starts_with("POST /oauth/token"))
        .count();
    assert_eq!(jetons, 1, "le jeton est demandé une fois : {journal:?}");
    assert!(journal
        .iter()
        .any(|l| l.contains("grant_type=account_credentials") && l.contains("account_id=compte")));
}

#[tokio::test]
async fn une_adresse_deja_inscrite_se_retrouve_dans_la_liste() {
    let (adresse, _) = serveur(std::sync::Arc::new(api)).await;
    let inscrit = client(&adresse)
        .inscrire("85746065432", "awa.diallo@exemple.org", "Awa", "Diallo")
        .await
        .unwrap();
    assert_eq!(inscrit.registrant_id, "reg-1");
    assert_eq!(
        inscrit.join_url.as_deref(),
        Some("https://zoom.us/w/1?tk=tk-1")
    );
}

#[tokio::test]
async fn un_quota_depasse_est_une_erreur_passagere() {
    let (adresse, _) = serveur(std::sync::Arc::new(api)).await;
    let erreur = client(&adresse)
        .inscrire("1", "a@exemple.org", "A", "B")
        .await
        .unwrap_err();
    assert!(erreur.est_passagere(), "{erreur}");
    assert_eq!(erreur.reessayer_dans(), Some(120));
}

#[tokio::test]
async fn un_refus_de_zoom_nest_pas_passager() {
    let (adresse, _) = serveur(std::sync::Arc::new(api)).await;
    let erreur = client(&adresse).lire_reunion("999").await.unwrap_err();
    assert!(
        matches!(
            erreur,
            ZoomError::Refusee {
                statut: 404,
                code: Some(3001),
                ..
            }
        ),
        "{erreur:?}"
    );
}
