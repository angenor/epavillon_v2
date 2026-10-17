//! Les routes des réunions Zoom sont montées dans la vraie application : sans
//! session, l'accès et le back-office répondent 401 (une route absente
//! répondrait 404), la lecture publique 404 sans réunion, et la route des
//! notifications n'existe pas sans configuration Zoom.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

macro_rules! application {
    () => {{
        let base = TestDb::new().await;
        let config = kernel::testing::test_config(base.url());
        let etat = api::state::AppState::new(base.db(), config)
            .await
            .expect("état de l'application");
        std::mem::forget(base);
        test::init_service(api::build_app(&etat)).await
    }};
}

#[actix_web::test]
async fn les_routes_de_reunion_sont_montees() {
    let app = application!();
    let seance = Uuid::now_v7();

    let acces = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/sessions/{seance}/meeting/access"))
            .to_request(),
    )
    .await;
    assert_eq!(acces.status(), StatusCode::UNAUTHORIZED);

    let admin = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/sessions/{seance}/meeting"))
            .set_json(serde_json::json!({}))
            .to_request(),
    )
    .await;
    assert_eq!(admin.status(), StatusCode::UNAUTHORIZED);

    let publique = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/sessions/{seance}/meeting"))
            .to_request(),
    )
    .await;
    assert_eq!(publique.status(), StatusCode::NOT_FOUND);

    let notification = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/webhooks/zoom")
            .set_payload("{}")
            .to_request(),
    )
    .await;
    assert_eq!(
        notification.status(),
        StatusCode::NOT_FOUND,
        "sans ZOOM_WEBHOOK_SECRET_TOKEN la route n'est pas montée"
    );
}
