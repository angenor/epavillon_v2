//! `GET /legal/{cle}` **à travers HTTP** : sans session, la même version pour
//! le site et pour l'application, le repli sur le français, l'empreinte, et un
//! texte que l'IFDD n'a pas encore fourni qui le dit.

use actix_web::http::header::{CONTENT_LANGUAGE, ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::testing::TestDb;
use serde_json::Value;

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    (base, etat)
}

#[actix_web::test]
async fn un_texte_se_lit_sans_session_et_porte_la_version_des_consentements() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    for cle in ["privacy", "terms"] {
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/legal/{cle}"))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK, "{cle}");
        let corps: Value = test::read_body_json(reponse).await;
        assert_eq!(corps["key"], cle);
        assert_eq!(corps["version"], kernel::legal::version(cle));
        // Le texte de l'IFDD n'est pas encore fourni : la route le dit, sans corps.
        assert_eq!(corps["status"], "pending");
        assert!(corps["body"].is_null());
    }
}

#[actix_web::test]
async fn la_langue_servie_est_dite_et_retombe_sur_le_francais() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let anglais = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/legal/privacy")
            .insert_header(("Accept-Language", "en"))
            .to_request(),
    )
    .await;
    assert_eq!(anglais.headers().get(CONTENT_LANGUAGE).unwrap(), "en");

    let espagnol = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/legal/privacy")
            .insert_header(("Accept-Language", "es"))
            .to_request(),
    )
    .await;
    assert_eq!(espagnol.headers().get(CONTENT_LANGUAGE).unwrap(), "fr");
    let corps: Value = test::read_body_json(espagnol).await;
    assert_eq!(corps["locale"], "fr");
}

#[actix_web::test]
async fn letag_rend_304_et_une_cle_inconnue_404() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let lue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/legal/terms")
            .to_request(),
    )
    .await;
    let empreinte = lue
        .headers()
        .get(ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/legal/terms")
            .insert_header((IF_NONE_MATCH, format!("W/{empreinte}")))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);

    let inconnue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/legal/cookies")
            .to_request(),
    )
    .await;
    assert_eq!(inconnue.status(), StatusCode::NOT_FOUND);
}
