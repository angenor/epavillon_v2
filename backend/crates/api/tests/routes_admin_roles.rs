//! Les quatre écritures du back-office, **à travers HTTP**.
//!
//! C'est la première obligation du principe X, et elle ne se tient pas au niveau
//! du service : le verbe, le préfixe `/api`, la personne prise dans l'URL plutôt
//! que dans le corps, et surtout **les six issues rendues en 200** ne vivent que
//! dans la couche route. Un refus prévu par le contrat du site n'est pas une
//! erreur HTTP — mais l'absence de session, elle, en est une.

use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::Value;
use uuid::Uuid;

const ADRESSE: &str = "patronne@example.org";
const MOT_DE_PASSE: &str = "Belem2027!";

struct Bac {
    base: TestDb,
    etat: AppState,
    cible: Uuid,
    cop: Uuid,
}

/// Une administratrice globale, une personne à qui confier un rôle, une édition
/// pour donner une portée aux attributions.
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

    let patron: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Aïcha', 'Bakayoko', now())
         RETURNING id",
    )
    .bind(ADRESSE)
    .fetch_one(base.pool())
    .await
    .expect("insertion de l'administratrice");

    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(patron)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("insertion du compte");

    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
         VALUES ($1, 'super_admin', 'global')",
    )
    .bind(patron)
    .execute(base.pool())
    .await
    .expect("attribution du rôle");

    let cible: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ('awa.diallo@example.org'::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de la personne visée");

    let cop: Uuid = sqlx::query_scalar(
        "INSERT INTO event.events
             (edition_year, title, slug, description, participation_mode,
              timezone, starts_at, ends_at)
         VALUES (2027, jsonb_build_object('fr', 'COP31 Belém'), 'cop31-belem'::text::platform.slug,
                 jsonb_build_object('fr', 'COP31 Belém'), 'online',
                 'America/Belem'::platform.timezone_name,
                 now() + interval '30 days', now() + interval '40 days')
         RETURNING id",
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de l'édition");

    Bac {
        base,
        etat,
        cible,
        cop,
    }
}

macro_rules! avec_session {
    ($app:expr, $requete:expr, $cookies:expr) => {{
        let mut requete = $requete;
        for cookie in $cookies.iter() {
            requete = requete.cookie(cookie.clone());
        }
        test::call_service(&$app, requete.to_request()).await
    }};
}

#[actix_web::test]
async fn le_cycle_dattribution_a_travers_http() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": ADRESSE, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    assert_eq!(connexion.status(), StatusCode::OK);
    let cookies: Vec<_> = connexion
        .response()
        .cookies()
        .map(|c| c.into_owned())
        .collect();

    // Les options d'attribution, avant tout : c'est ce que l'écran lit pour
    // composer son panneau.
    let reponse = avec_session!(
        app,
        test::TestRequest::get().uri("/api/admin/users/role-options"),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let options: Value = test::read_body_json(reponse).await;
    assert_eq!(options["can_assign_global"], true);
    assert!(options["events"].as_array().expect("éditions").len() == 1);

    // L'attribution. `person_id` du corps désigne quelqu'un d'autre : c'est
    // l'URL qui fait foi, et le corps est ignoré (FR-055).
    let reponse = avec_session!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/users/{}/roles", bac.cible))
            .set_json(serde_json::json!({
                "person_id": Uuid::now_v7(),
                "role_code": "reviewer",
                "scope_type": "event",
                "scope_id": bac.cop,
                "note": "comité de sélection",
                "granted": [{ "permission_code": "identity.role.assign" }]
            })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "granted");
    assert_eq!(corps["assignment"]["person_id"], bac.cible.to_string());
    let attribution = corps["assignment"]["id"]
        .as_str()
        .expect("identifiant de l'attribution")
        .to_owned();

    // Le rôle a bien été posé sur la personne de l'URL, et sur personne d'autre.
    let posees: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity.role_assignments WHERE role_code = 'reviewer'",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(posees, 1);

    // Le doublon : refusé, **en 200**, avec la ligne en conflit.
    let reponse = avec_session!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/users/{}/roles", bac.cible))
            .set_json(serde_json::json!({
                "role_code": "reviewer",
                "scope_type": "event",
                "scope_id": bac.cop
            })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "duplicate");
    assert_eq!(corps["conflict_with"]["id"], attribution);

    // Le retrait. `DELETE` par le verbe, mais la ligne reste.
    let reponse = avec_session!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/admin/users/roles/{attribution}"))
            .set_json(serde_json::json!({ "reason": "fin de mission" })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "revoked");
    assert_eq!(corps["assignment"]["revoked_reason"], "fin de mission");

    let survivante: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity.role_assignments WHERE role_code = 'reviewer'",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(survivante, 1, "le retrait ne supprime jamais");
}

#[actix_web::test]
async fn une_suspension_sans_terme_est_refusee_puis_acceptee_avec() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": ADRESSE, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    let cookies: Vec<_> = connexion
        .response()
        .cookies()
        .map(|c| c.into_owned())
        .collect();

    let reponse = avec_session!(
        app,
        test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", bac.cible))
            .set_json(serde_json::json!({
                "status": "suspended",
                "reason": "propos déplacés",
                "suspended_until": null,
                "revoke_sessions": true
            })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(
        corps["status"], "missing_deadline",
        "la base refuse une suspension sans terme, et le refus sort en 200"
    );

    let inchange: String =
        sqlx::query_scalar("SELECT status::text FROM identity.people WHERE id = $1")
            .bind(bac.cible)
            .fetch_one(bac.base.pool())
            .await
            .expect("relecture");
    assert_eq!(inchange, "active");

    let reponse = avec_session!(
        app,
        test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", bac.cible))
            .set_json(serde_json::json!({
                "status": "suspended",
                "reason": "propos déplacés",
                "suspended_until": "2027-06-01T00:00:00Z",
                "revoke_sessions": true
            })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "saved");
    assert_eq!(corps["detail"]["status"], "suspended");
    assert_eq!(corps["detail"]["status_reason"], "propos déplacés");
    assert_eq!(corps["detail"]["status_changed_by_name"], "Aïcha Bakayoko");

    let evenements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.outbox_events
          WHERE event_type = 'identity.person.status_changed'",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(evenements, 1);
}

/// **Un compte ordinaire ne sonde pas cette route.** `forbidden_scope` répond à
/// un administrateur qui vise la mauvaise portée ; à qui n'a aucun droit
/// d'attribution, la route rend **403** — sans quoi n'importe quel compte
/// connecté lirait les rôles de n'importe qui en tentant une écriture vouée à
/// l'échec.
#[actix_web::test]
async fn un_compte_sans_droit_dattribution_recoit_403_et_rien_dautre() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");
    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(bac.cible)
    .bind(&empreinte)
    .execute(bac.base.pool())
    .await
    .expect("compte de la personne visée");

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "awa.diallo@example.org",
                "password": MOT_DE_PASSE
            }))
            .to_request(),
    )
    .await;
    assert_eq!(connexion.status(), StatusCode::OK);
    let cookies: Vec<_> = connexion
        .response()
        .cookies()
        .map(|c| c.into_owned())
        .collect();

    let reponse = avec_session!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/users/{}/roles", bac.cible))
            .set_json(serde_json::json!({
                "role_code": "super_admin",
                "scope_type": "global"
            })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::FORBIDDEN);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "FORBIDDEN");
    assert!(
        corps.get("assignments").is_none(),
        "aucun rôle ne franchit un refus d'autorisation"
    );
}

/// Une valeur hors liste nomme **la valeur**, pas un champ : le refus ne doit
/// donc désigner aucune case. `anonymized` se posait comme champ fautif, et
/// l'écran aurait souligné une case qui n'existe pas.
#[actix_web::test]
async fn le_statut_deffacement_est_refuse_sans_designer_de_champ_fantome() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": ADRESSE, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    let cookies: Vec<_> = connexion
        .response()
        .cookies()
        .map(|c| c.into_owned())
        .collect();

    let reponse = avec_session!(
        app,
        test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", bac.cible))
            .set_json(serde_json::json!({ "status": "anonymized", "reason": "RGPD" })),
        cookies
    );
    assert_eq!(reponse.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "VALIDATION_FAILED");
    assert!(
        corps["field"].is_null(),
        "« anonymized » est une valeur, pas un champ : {}",
        corps["field"]
    );
}

/// Sans session, une écriture d'administration ne rend pas un discriminant :
/// elle rend **401**. Le contrat ne prévoit un refus en 200 que pour ce que
/// l'écran sait présenter — et un écran d'administration fermé n'en fait pas
/// partie.
#[actix_web::test]
async fn sans_session_les_ecritures_de_role_rendent_401() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    for requete in [
        test::TestRequest::get().uri("/api/admin/users/role-options"),
        test::TestRequest::post()
            .uri(&format!("/api/admin/users/{}/roles", bac.cible))
            .set_json(serde_json::json!({ "role_code": "reviewer", "scope_type": "global" })),
        test::TestRequest::put()
            .uri(&format!("/api/admin/users/{}/status", bac.cible))
            .set_json(serde_json::json!({ "status": "blocked", "reason": "x" })),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::UNAUTHORIZED);
    }
}
