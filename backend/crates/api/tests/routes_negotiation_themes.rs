//! Les thématiques suivies, **à travers HTTP** : que les deux routes sont
//! montées sous `/api` et gardées par la session — URL forgée comprise —, que
//! l'`ETag` circule, que `If-None-Match` rend 304 et `If-Match` périmé 412.

use actix_web::http::header::{CACHE_CONTROL, ETAG, IF_MATCH, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::{json, Value};
use uuid::Uuid;

const ADRESSE: &str = "awa.diallo@example.org";
const MOT_DE_PASSE: &str = "Belem2027!";

struct Bac {
    base: TestDb,
    etat: AppState,
    person_id: Uuid,
}

async fn monter() -> Bac {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");

    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");

    let person_id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(ADRESSE)
    .fetch_one(base.pool())
    .await
    .expect("insertion de la personne");

    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(person_id)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("insertion du compte");

    Bac {
        base,
        etat,
        person_id,
    }
}

fn corps_de_connexion() -> Value {
    json!({
        "email": ADRESSE,
        "password": MOT_DE_PASSE,
        "remember_me": false,
        "client": { "kind": "app", "device_id": "9f2c-appareil", "platform": "android" },
    })
}

macro_rules! se_connecter {
    ($app:expr) => {{
        let reponse = test::call_service(
            &$app,
            test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(corps_de_connexion())
                .to_request(),
        )
        .await;

        assert_eq!(reponse.status(), StatusCode::OK);
        reponse
            .response()
            .cookies()
            .find(|c| c.name() == "epavillon_at")
            .map(|c| format!("{}={}", c.name(), c.value()))
            .expect("cookie d'accès")
    }};
}

fn etag_de<B>(reponse: &actix_web::dev::ServiceResponse<B>) -> String {
    reponse
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag lisible")
        .to_owned()
}

#[actix_web::test]
async fn sans_session_les_deux_routes_refusent_url_forgee_comprise() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let lecture = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/themes")
            .to_request(),
    )
    .await;
    assert_eq!(lecture.status(), StatusCode::UNAUTHORIZED);

    // Une URL forgée qui nomme une personne : la session seule dit qui parle.
    let forgee = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/negotiation/me/themes?person_id={}",
                bac.person_id
            ))
            .to_request(),
    )
    .await;
    assert_eq!(forgee.status(), StatusCode::UNAUTHORIZED);

    let ecriture = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/negotiation/me/themes")
            .set_json(json!({ "codes": ["adaptation"] }))
            .to_request(),
    )
    .await;
    assert_eq!(ecriture.status(), StatusCode::UNAUTHORIZED);

    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM negotiation.theme_subscriptions")
        .fetch_one(bac.base.pool())
        .await
        .expect("compte");
    assert_eq!(n, 0);
}

#[actix_web::test]
async fn letag_le_304_et_le_412_de_bout_en_bout() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    let vide = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(vide.status(), StatusCode::OK);
    let empreinte_vide = etag_de(&vide);
    let corps: Value = test::read_body_json(vide).await;
    assert_eq!(corps["themes"], json!([]));

    let pose = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie.clone()))
            .insert_header((IF_MATCH, empreinte_vide.clone()))
            .set_json(json!({ "codes": ["gender", "adaptation"] }))
            .to_request(),
    )
    .await;
    assert_eq!(pose.status(), StatusCode::OK);
    let empreinte = etag_de(&pose);
    assert_ne!(empreinte, empreinte_vide);
    let corps: Value = test::read_body_json(pose).await;
    assert_eq!(corps["themes"][0]["code"], "adaptation");
    assert_eq!(corps["themes"][1]["code"], "gender");
    assert!(corps["themes"][0]["followed_at"].is_string());

    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie.clone()))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchange), empreinte);

    // Le téléphone rejoue l'empreinte de l'état vide : trop tard.
    let en_retard = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie.clone()))
            .insert_header((IF_MATCH, empreinte_vide))
            .set_json(json!({ "codes": ["finance"] }))
            .to_request(),
    )
    .await;
    assert_eq!(en_retard.status(), StatusCode::PRECONDITION_FAILED);
    let corps: Value = test::read_body_json(en_retard).await;
    assert_eq!(corps["code"], "NEGOTIATION_THEMES_STALE");

    let relu = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie))
            .to_request(),
    )
    .await;
    assert_eq!(etag_de(&relu), empreinte, "l'état n'a pas bougé");
}

/// Derrière un relais qui compresse, l'empreinte revient réécrite : elle désigne
/// toujours le même état. Et l'état d'une personne ne se garde dans aucun cache
/// partagé.
#[actix_web::test]
async fn le_304_resiste_au_relais_et_rien_ne_se_garde_en_cache_partage() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    let lue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(
        lue.headers()
            .get(CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("private, no-cache")
    );
    let empreinte = etag_de(&lue);
    let nu = empreinte.trim_matches('"').to_owned();

    for presentee in [format!("W/\"{nu}-gzip\""), format!("\"{nu}-br\"")] {
        let inchange = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/negotiation/me/themes")
                .insert_header(("cookie", cookie.clone()))
                .insert_header((IF_NONE_MATCH, presentee.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED, "{presentee}");
        assert_eq!(
            inchange
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some("private, no-cache")
        );
    }

    let acces = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/access")
            .insert_header(("cookie", cookie))
            .to_request(),
    )
    .await;
    assert_eq!(acces.status(), StatusCode::OK);
    assert_eq!(
        acces
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("private, no-cache")
    );
}

#[actix_web::test]
async fn un_code_inconnu_sort_en_400_et_nomme_le_code() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    let reponse = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/negotiation/me/themes")
            .insert_header(("cookie", cookie))
            .set_json(json!({ "codes": ["biodiversity"] }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::BAD_REQUEST);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "NEGOTIATION_THEME_UNKNOWN");
    assert!(corps["message"]
        .as_str()
        .is_some_and(|m| m.contains("« biodiversity »")));
}
