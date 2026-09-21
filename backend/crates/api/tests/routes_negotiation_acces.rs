//! Les deux routes de l'admission, **à travers HTTP**.
//!
//! Ce que les tests du crate `negotiation` ne peuvent pas prouver : que la route
//! est réellement montée sous `/api`, que l'extracteur de session la garde, que
//! la session de l'appelant remonte jusqu'à l'usage du code, que l'`ETag` est
//! là, et surtout **qu'un refus sort bien en 200** et non en 4xx — le service
//! rend une valeur, c'est la couche route qui choisit le statut.

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::Value;
use uuid::Uuid;

const ADRESSE: &str = "awa.diallo@example.org";
const MOT_DE_PASSE: &str = "Belem2027!";
const CODE: &str = "NEGO-001";

struct Bac {
    base: TestDb,
    etat: AppState,
    space_id: Uuid,
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

    let space_id: Uuid = sqlx::query_scalar(
        "INSERT INTO negotiation.spaces (slug, name, visibility)
         VALUES ('cop31-http'::text::platform.slug,
                 jsonb_build_object('fr', 'COP31 — Climat'), 'listed')
         RETURNING id",
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de l'espace");

    let terme: Uuid = sqlx::query_scalar(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_network' AND code = 'women_negotiators'",
    )
    .fetch_one(base.pool())
    .await
    .expect("terme du réseau");

    sqlx::query(
        "INSERT INTO negotiation.invitation_codes
             (code, label, scope_type, space_id, grants_network_term_id)
         VALUES ($1, 'Réseau des négociatrices — COP31', 'negotiation_space', $2, $3)",
    )
    .bind(CODE)
    .bind(space_id)
    .bind(terme)
    .execute(base.pool())
    .await
    .expect("insertion du code");

    Bac {
        base,
        etat,
        space_id,
    }
}

/// Le corps de connexion de l'application : **avec son objet `client`**, comme
/// Guide Négo l'envoie.
fn corps_de_connexion() -> Value {
    serde_json::json!({
        "email": ADRESSE,
        "password": MOT_DE_PASSE,
        "remember_me": false,
        "client": { "kind": "app", "device_id": "9f2c-appareil", "platform": "android" },
    })
}

/// Le cookie d'accès, obtenu comme l'application l'obtient.
///
/// Écrit en macro et non en fonction : le type de requête de l'application
/// montée vient d'`actix-http`, que ce crate n'a pas en dépendance, et l'ajouter
/// pour une signature de test serait une dépendance de trop.
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

#[actix_web::test]
async fn sans_session_les_deux_routes_refusent() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let lecture = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/access")
            .to_request(),
    )
    .await;
    assert_eq!(lecture.status(), StatusCode::UNAUTHORIZED);

    let saisie = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/negotiation/invitation-codes/redeem")
            .set_json(serde_json::json!({ "code": CODE }))
            .to_request(),
    )
    .await;
    assert_eq!(saisie.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn le_parcours_complet_dune_entree() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    // Avant le code : visiteuse, et le mode d'admission dit quoi proposer.
    let avant = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/access")
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(avant.status(), StatusCode::OK);
    let empreinte = avant
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag lisible")
        .to_owned();
    let corps: Value = test::read_body_json(avant).await;
    assert_eq!(corps["state"], "visitor");
    assert_eq!(corps["admission_mode"], "code");
    assert!(corps["granted"].is_null());

    // La même empreinte présentée : rien n'a changé, et le corps ne repart pas.
    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/access")
            .insert_header(("cookie", cookie.clone()))
            .insert_header((IF_NONE_MATCH, empreinte))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);

    // Le code, écrit comme il circule : minuscules et espace.
    let saisie = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/negotiation/invitation-codes/redeem")
            .insert_header(("cookie", cookie.clone()))
            .set_json(serde_json::json!({ "code": "nego 001", "device_id": "9f2c-appareil" }))
            .to_request(),
    )
    .await;
    assert_eq!(saisie.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(saisie).await;
    assert_eq!(corps["issue"], "accepted");
    assert!(
        corps["message"]
            .as_str()
            .is_some_and(|m| m.contains("Code reconnu")),
        "{corps}"
    );
    assert_eq!(corps["granted"]["scope"]["name"], "COP31 — Climat");
    assert_eq!(corps["networks"][0]["code"], "women_negotiators");

    // **La session de l'appelant est gardée avec l'usage** : c'est ce qui dira
    // plus tard d'où une entrée est partie.
    let session_de_lusage: Option<Uuid> =
        sqlx::query_scalar("SELECT session_id FROM negotiation.invitation_code_uses")
            .fetch_one(bac.base.pool())
            .await
            .expect("lecture de l'usage");
    assert!(
        session_de_lusage.is_some(),
        "l'usage doit porter la session d'où le code a été saisi"
    );

    // Après le code : admise, avec ce que son accès ouvre.
    let apres = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/negotiation/me/access")
            .insert_header(("cookie", cookie))
            .to_request(),
    )
    .await;
    let corps: Value = test::read_body_json(apres).await;
    assert_eq!(corps["state"], "granted");
    assert_eq!(corps["granted"]["scope"]["type"], "negotiation_space");
    assert_eq!(
        corps["granted"]["scope"]["id"],
        bac.space_id.to_string().as_str()
    );
    assert_eq!(
        corps["granted"]["source_code_label"],
        "Réseau des négociatrices — COP31"
    );
}

/// **La politique de statut**, éprouvée là où elle vit : un refus prévu par le
/// parcours sort en 200 avec son discriminant. En 4xx, le transport du client le
/// traiterait comme une panne et l'écran perdrait ses sorties.
#[actix_web::test]
async fn un_code_inconnu_sort_en_200() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/negotiation/invitation-codes/redeem")
            .insert_header(("cookie", cookie))
            .set_json(serde_json::json!({ "code": "ZZZZ-999" }))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["issue"], "unknown");
    assert!(corps["granted"].is_null());
}

/// Un corps vide, lui, reste une vraie erreur : le champ fautif est nommé.
#[actix_web::test]
async fn un_code_vide_sort_en_erreur_de_validation() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app);

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/negotiation/invitation-codes/redeem")
            .insert_header(("cookie", cookie))
            .set_json(serde_json::json!({ "code": "  " }))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["field"], "code");
}
