//! La plateforme en maintenance (`platform.maintenance`), à travers HTTP : la
//! bascule depuis le back-office, et le verrou qui ne laisse écrire que les
//! personnes qui administrent.

use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::{json, Value};
use uuid::Uuid;

const MOT_DE_PASSE: &str = "Belem2027!";

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    (base, etat)
}

/// Une personne au rôle global donné, et son cookie de session.
async fn connexion(base: &TestDb, etat: &AppState, adresse: &str, role: &str) -> String {
    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");

    let personne: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(adresse)
    .fetch_one(base.pool())
    .await
    .expect("insertion de la personne");

    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(personne)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("insertion du compte");

    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
         VALUES ($1, $2, 'global')",
    )
    .bind(personne)
    .bind(role)
    .execute(base.pool())
    .await
    .expect("attribution du rôle");

    let app = test::init_service(api::build_app(etat)).await;
    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": adresse, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);

    reponse
        .response()
        .cookies()
        .find(|c| c.name() == identity::COOKIE_ACCES)
        .map(|c| format!("{}={}", c.name(), c.value()))
        .expect("cookie d'accès")
}

async fn basculer(etat: &AppState, cookie: &str, enabled: bool) -> (StatusCode, Value) {
    let app = test::init_service(api::build_app(etat)).await;
    let reponse = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/admin/platform/maintenance")
            .insert_header(("Cookie", cookie.to_owned()))
            .set_json(json!({ "enabled": enabled }))
            .to_request(),
    )
    .await;
    let statut = reponse.status();
    (statut, test::read_body_json(reponse).await)
}

/// Une écriture quelconque hors connexion : la création d'une organisation.
async fn ecrire(etat: &AppState, cookie: Option<&str>) -> (StatusCode, Value) {
    let app = test::init_service(api::build_app(etat)).await;
    let mut requete = test::TestRequest::post()
        .uri("/api/organizations")
        .set_json(json!({}));
    if let Some(cookie) = cookie {
        requete = requete.insert_header(("Cookie", cookie.to_owned()));
    }
    let reponse = test::call_service(&app, requete.to_request()).await;
    let statut = reponse.status();
    let corps = serde_json::from_slice(&test::read_body(reponse).await).unwrap_or(Value::Null);
    (statut, corps)
}

#[tokio::test]
async fn seul_un_administrateur_global_bascule_la_maintenance() {
    let (base, etat) = monter().await;
    let quidam = connexion(&base, &etat, "quidam@exemple.org", "standard").await;

    let (statut, _) = basculer(&etat, &quidam, true).await;
    assert_eq!(statut, StatusCode::FORBIDDEN);

    let admin = connexion(&base, &etat, "admin@francophonie.org", "admin").await;
    let (statut, corps) = basculer(&etat, &admin, true).await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["enabled"], true);

    // Résolu sans personne : le site le lit avant de savoir qui regarde.
    let app = test::init_service(api::build_app(&etat)).await;
    let drapeaux: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri("/api/platform/feature-flags")
            .to_request(),
    )
    .await;
    let maintenance = drapeaux
        .as_array()
        .and_then(|d| d.iter().find(|f| f["key"] == "platform.maintenance"))
        .expect("drapeau semé");
    assert_eq!(maintenance["is_enabled"], true);
}

#[tokio::test]
async fn en_maintenance_seuls_les_administrateurs_ecrivent() {
    let (base, etat) = monter().await;
    let admin = connexion(&base, &etat, "admin@francophonie.org", "admin").await;
    let quidam = connexion(&base, &etat, "quidam@exemple.org", "standard").await;
    basculer(&etat, &admin, true).await;

    let (statut, corps) = ecrire(&etat, None).await;
    assert_eq!(statut, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(corps["code"], "PLATFORM_MAINTENANCE");

    let (statut, corps) = ecrire(&etat, Some(&quidam)).await;
    assert_eq!(statut, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(corps["code"], "PLATFORM_MAINTENANCE");

    // L'administrateur passe le verrou : la route répond sur le fond.
    let (statut, _) = ecrire(&etat, Some(&admin)).await;
    assert_ne!(statut, StatusCode::SERVICE_UNAVAILABLE);

    // La connexion reste ouverte, sans quoi personne n'entrerait.
    let app = test::init_service(api::build_app(&etat)).await;
    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": "inconnu@exemple.org", "password": "x" }))
            .to_request(),
    )
    .await;
    assert_ne!(reponse.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Rouverte, la plateforme reprend les écritures de tous.
    basculer(&etat, &admin, false).await;
    let (statut, _) = ecrire(&etat, Some(&quidam)).await;
    assert_ne!(statut, StatusCode::SERVICE_UNAVAILABLE);
}
