//! Le paquet du savoir, **à travers HTTP** : monté sous `/api`, ouvert sans
//! session, son `ETag` public qui suit la langue, le `304`, et `since` illisible.

use actix_web::http::header::{ACCEPT_LANGUAGE, CACHE_CONTROL, ETAG, IF_NONE_MATCH, VARY};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::testing::TestDb;
use serde_json::Value;

const PAQUET: &str = "/api/negotiation/knowledge";

fn entete<B>(
    reponse: &actix_web::dev::ServiceResponse<B>,
    nom: actix_web::http::header::HeaderName,
) -> String {
    reponse
        .headers()
        .get(nom)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

#[actix_web::test]
async fn le_paquet_est_public_porte_son_empreinte_et_rend_304() {
    let base = TestDb::new().await;
    let etat = AppState::new(base.db(), kernel::testing::test_config(base.url()))
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let entier = test::call_service(&app, test::TestRequest::get().uri(PAQUET).to_request()).await;
    assert_eq!(entier.status(), StatusCode::OK);
    assert_eq!(entete(&entier, CACHE_CONTROL), "public, no-cache");
    assert!(entete(&entier, VARY).contains("Accept-Language"));
    let empreinte = entete(&entier, ETAG);
    assert_eq!(empreinte.len(), 34);
    let corps: Value = test::read_body_json(entier).await;
    assert_eq!(corps["complete"], true);
    for champ in [
        "served_at",
        "faq_sections",
        "glossary_families",
        "faq",
        "glossary",
        "pathway",
        "most_read",
        "removed",
    ] {
        assert!(corps.get(champ).is_some(), "{champ} manque");
    }

    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PAQUET)
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(entete(&inchange, ETAG), empreinte);
    assert_eq!(entete(&inchange, CACHE_CONTROL), "public, no-cache");

    let anglais = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PAQUET)
            .insert_header((ACCEPT_LANGUAGE, "en"))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(
        anglais.status(),
        StatusCode::OK,
        "une copie française ne vaut pas en anglais"
    );

    let served_at = corps["served_at"].as_str().unwrap().replace('+', "%2B");
    let difference = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("{PAQUET}?since={served_at}"))
            .to_request(),
    )
    .await;
    assert_eq!(difference.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(difference).await;
    assert_eq!(corps["complete"], false);

    let illisible = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("{PAQUET}?since=hier"))
            .to_request(),
    )
    .await;
    assert_eq!(illisible.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let corps: Value = test::read_body_json(illisible).await;
    assert_eq!(corps["field"], "since");
}
