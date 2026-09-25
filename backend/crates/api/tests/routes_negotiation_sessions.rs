//! Les sessions officielles, les groupes et l'agenda **à travers HTTP** : montés
//! sous `/api`, la lecture des sessions publique avec son `304`, et les routes
//! « me » refusées sans compte.

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::testing::TestDb;
use serde_json::{json, Value};
use uuid::Uuid;

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    (base, etat)
}

#[actix_web::test]
async fn sans_compte_les_routes_me_refusent() {
    let (base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;
    let session = Uuid::now_v7();

    for requete in [
        test::TestRequest::get().uri("/api/negotiation/me/groups"),
        test::TestRequest::put()
            .uri("/api/negotiation/me/groups")
            .set_json(json!({ "groups": ["ldc"] })),
        test::TestRequest::get().uri("/api/negotiation/me/agenda"),
        test::TestRequest::put()
            .uri(&format!("/api/negotiation/me/agenda/{session}"))
            .set_json(json!({ "remind": true })),
        test::TestRequest::delete().uri(&format!("/api/negotiation/me/agenda/{session}")),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::UNAUTHORIZED);
    }

    let n: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM negotiation.group_subscriptions)
              + (SELECT count(*) FROM negotiation.agenda_entries)",
    )
    .fetch_one(base.pool())
    .await
    .expect("compte");
    assert_eq!(n, 0);
}

#[actix_web::test]
async fn la_lecture_des_sessions_est_publique_et_rend_304() {
    let (base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let inconnue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/sessions?edition=cop99")
            .to_request(),
    )
    .await;
    assert_eq!(inconnue.status(), StatusCode::NOT_FOUND);
    let corps: Value = test::read_body_json(inconnue).await;
    assert_eq!(corps["code"], "NEGOTIATION_EDITION_UNKNOWN");

    let edition: Uuid = sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
           VALUES (2026, '{"fr":"COP31"}'::jsonb, 'cop31', '{"fr":"Description."}'::jsonb,
                   'online', 'Asia/Istanbul', '2026-11-09T00:00:00Z', '2026-11-20T00:00:00Z')
           RETURNING id"#,
    )
    .fetch_one(base.pool())
    .await
    .expect("édition");
    sqlx::query(
        "INSERT INTO negotiation.official_imports (event_id, official_programme_url)
         VALUES ($1, 'https://unfccc.int/cop31/schedule')",
    )
    .bind(edition)
    .execute(base.pool())
    .await
    .expect("import éteint");

    let lecture = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/sessions?edition=cop31")
            .to_request(),
    )
    .await;
    assert_eq!(lecture.status(), StatusCode::OK);
    let empreinte = lecture
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("lisible")
        .to_owned();
    let corps: Value = test::read_body_json(lecture).await;
    assert_eq!(corps["state"], "cut");
    assert_eq!(corps["cut_reason"], "disabled");
    assert_eq!(corps["sessions"], json!([]));
    assert_eq!(corps["edition"]["timezone"], "Asia/Istanbul");

    let relecture = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/sessions?edition=cop31")
            .insert_header((IF_NONE_MATCH, empreinte))
            .to_request(),
    )
    .await;
    assert_eq!(relecture.status(), StatusCode::NOT_MODIFIED);
}
