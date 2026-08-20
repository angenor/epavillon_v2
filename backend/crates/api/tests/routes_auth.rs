//! Le chemin nominal de chaque route livrée, **à travers HTTP**.
//!
//! C'est la première obligation du principe X, et elle ne se tient pas au
//! niveau du service : les statuts, les corps d'union, le préfixe `/api`, les
//! attributs des deux cookies et la traduction d'un corps mal formé ne vivent
//! que dans la couche route. Un test de service les laisse tous sans couverture,
//! et une inversion de `SameSite` ou un 401 rendu à la place d'un 200 resterait
//! verte jusqu'au raccordement du site.
//!
//! L'application montée ici est **celle du binaire** — `api::build_app`, avec
//! ses trois intergiciels —, sur une base jetable.

use actix_web::cookie::{Cookie, SameSite};
use actix_web::http::header::ORIGIN;
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::Value;
use sqlx::types::ipnetwork::IpNetwork;

const ADRESSE: &str = "awa.diallo@example.org";
const MOT_DE_PASSE: &str = "Belem2027!";

struct Bac {
    base: TestDb,
    etat: AppState,
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

    let person_id: uuid::Uuid = sqlx::query_scalar(
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

    Bac { base, etat }
}

fn corps_de_connexion(mot_de_passe: &str) -> Value {
    serde_json::json!({
        "email": ADRESSE,
        "password": mot_de_passe,
        "remember_me": false,
    })
}

#[actix_web::test]
async fn ready_repond_sans_session() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/ready").to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);
}

#[actix_web::test]
async fn la_connexion_pose_deux_cookies_aux_bonnes_portees() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);

    let acces = cookie(&reponse, "epavillon_at").expect("cookie d'accès");
    assert!(acces.http_only().unwrap_or(false));
    assert_eq!(acces.same_site(), Some(SameSite::Lax));
    assert_eq!(acces.path(), Some("/"));
    assert!(acces
        .max_age()
        .expect("durée du cookie d'accès")
        .is_positive());

    // Le jeton de rafraîchissement ne va qu'aux routes de session : c'est ce qui
    // limite le dégât d'une fuite par une autre route (research.md § R2).
    let rafraichissement = cookie(&reponse, "epavillon_rt").expect("cookie de rafraîchissement");
    assert!(rafraichissement.http_only().unwrap_or(false));
    assert_eq!(rafraichissement.same_site(), Some(SameSite::Strict));
    assert_eq!(rafraichissement.path(), Some("/api/auth"));

    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "authenticated");
    assert_eq!(corps["person"]["primary_email"], ADRESSE);
}

/// La politique de statut du contrat : un refus prévu par le front sort en 200,
/// et ne pose aucun cookie.
#[actix_web::test]
async fn un_refus_sort_en_200_et_sans_cookie() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion("nimportequoi"))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);
    assert!(cookie(&reponse, "epavillon_at").is_none());
    assert!(cookie(&reponse, "epavillon_rt").is_none());

    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "invalid_credentials");
}

/// Le store du site appelle cette route à chaque navigation, y compris
/// déconnecté : un 401 y ferait afficher un écran en panne.
#[actix_web::test]
async fn me_rend_null_sans_session_et_la_personne_avec() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let anonyme = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/auth/me").to_request(),
    )
    .await;
    assert_eq!(anonyme.status(), StatusCode::OK);
    assert_eq!(test::read_body(anonyme).await.as_ref(), b"null");

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;
    let acces = cookie(&connexion, "epavillon_at").expect("cookie d'accès");

    let connecte = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .cookie(acces)
            .to_request(),
    )
    .await;
    assert_eq!(connecte.status(), StatusCode::OK);

    let corps: Value = test::read_body_json(connecte).await;
    assert_eq!(corps["primary_email"], ADRESSE);
}

#[actix_web::test]
async fn renouvellement_puis_deconnexion() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;
    let rafraichissement = cookie(&connexion, "epavillon_rt").expect("cookie de rafraîchissement");

    let renouvelee = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/refresh")
            .cookie(rafraichissement.clone())
            .to_request(),
    )
    .await;
    assert_eq!(renouvelee.status(), StatusCode::OK);
    let neuf = cookie(&renouvelee, "epavillon_rt").expect("jeton renouvelé");
    assert_ne!(neuf.value(), rafraichissement.value());
    let corps: Value = test::read_body_json(renouvelee).await;
    assert_eq!(corps["status"], "renewed");

    for _ in 0..2 {
        let deconnexion = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/auth/logout")
                .cookie(neuf.clone())
                .to_request(),
        )
        .await;
        assert_eq!(deconnexion.status(), StatusCode::OK);
        let corps: Value = test::read_body_json(deconnexion).await;
        assert_eq!(corps["status"], "signed_out");
    }

    let apres = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/refresh")
            .cookie(neuf)
            .to_request(),
    )
    .await;
    let corps: Value = test::read_body_json(apres).await;
    assert_eq!(corps["status"], "expired");
}

/// Le rejeu efface les deux cookies : les laisser ferait rejouer la même
/// détection à chaque appel du navigateur.
#[actix_web::test]
async fn le_rejeu_rend_401_et_efface_les_cookies() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;
    let vole = cookie(&connexion, "epavillon_rt").expect("cookie de rafraîchissement");

    let premier = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/refresh")
            .cookie(vole.clone())
            .to_request(),
    )
    .await;
    assert_eq!(premier.status(), StatusCode::OK);

    let rejeu = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/refresh")
            .cookie(vole)
            .to_request(),
    )
    .await;

    assert_eq!(rejeu.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        cookie(&rejeu, "epavillon_at")
            .expect("cookie d'accès effacé")
            .value(),
        ""
    );
    assert_eq!(
        cookie(&rejeu, "epavillon_rt")
            .expect("cookie de rafraîchissement effacé")
            .value(),
        ""
    );

    let corps: Value = test::read_body_json(rejeu).await;
    assert_eq!(corps["code"], "IDENTITY_REFRESH_REUSED");
}

/// Le catalogue promet un code stable, un message français et l'identifiant de
/// requête — y compris quand c'est la désérialisation qui refuse.
#[actix_web::test]
async fn un_corps_incomplet_rend_le_corps_du_catalogue() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": ADRESSE }))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "VALIDATION_FAILED");
    assert_eq!(corps["field"], "password");
    assert!(corps["request_id"].is_string());
    assert!(
        !corps["message"].as_str().unwrap().contains("missing field"),
        "le texte de serde ne franchit jamais la réponse"
    );
}

#[actix_web::test]
async fn une_ecriture_dorigine_inconnue_est_refusee() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header((ORIGIN, "https://attaquant.example"))
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::FORBIDDEN);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "IDENTITY_ORIGIN_REJECTED");
}

#[actix_web::test]
async fn un_chemin_inconnu_rend_le_corps_du_catalogue() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/nimporte-quoi")
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "NOT_FOUND");
}

/// **Deux statuts qui ne vivent que dans la couche route.** Un jeton refusé sort
/// en 200 avec son discriminant — l'écran propose de redemander un lien ; un mot
/// de passe refusé sort en 422 sur le champ `password` — le formulaire se
/// corrige sur place. Un service qui rendrait les deux de la même façon
/// laisserait l'écran sans moyen de les distinguer.
///
/// Le contrôle prend son jeton en **paramètre de requête** : c'est un `GET`, et
/// un `GET` avec un corps ne se met pas en cache et ne se documente pas.
#[actix_web::test]
async fn le_cycle_de_reinitialisation_a_travers_http() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/password-reset")
            .set_json(serde_json::json!({ "email": ADRESSE }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "sent");

    let jeton: String = sqlx::query_scalar(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_password_reset_email'
          ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi");

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/auth/password-reset/check?token={jeton}"))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "valid");
    assert_eq!(corps["email"], ADRESSE);

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/password-reset/confirm")
            .set_json(serde_json::json!({ "token": jeton, "password": "court" }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "IDENTITY_PASSWORD_TOO_WEAK");
    assert_eq!(corps["field"], "password");

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/password-reset/confirm")
            .set_json(serde_json::json!({ "token": jeton, "password": "Ouagadougou2027" }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "reset");

    // Le même lien, une seconde fois : refusé, mais **en 200** avec son motif.
    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/password-reset/confirm")
            .set_json(serde_json::json!({ "token": jeton, "password": "Ouagadougou2027" }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps["status"], "rejected");
    assert_eq!(corps["reason"], "already_used");

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(
        corps["status"], "invalid_credentials",
        "l'ancien mot de passe ne vaut plus rien"
    );
}

fn cookie<B>(reponse: &actix_web::dev::ServiceResponse<B>, nom: &str) -> Option<Cookie<'static>> {
    reponse
        .response()
        .cookies()
        .find(|c| c.name() == nom)
        .map(|c| c.into_owned())
}

/// `X-Forwarded-For` n'est cru que d'un mandataire déclaré. Sans
/// `TRUSTED_PROXIES` — le défaut —, c'est l'adresse du pair qui est enregistrée,
/// et un client qui annonce la sienne ne choisit rien.
#[actix_web::test]
async fn lentete_dadresse_dun_pair_inconnu_est_ignoree() {
    let bac = monter().await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .peer_addr("198.51.100.4:54321".parse().expect("socket de test"))
            .insert_header(("x-forwarded-for", "203.0.113.9"))
            .set_json(corps_de_connexion(MOT_DE_PASSE))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);

    let enregistree: Option<IpNetwork> = sqlx::query_scalar(
        "SELECT ip_address FROM identity.sessions ORDER BY issued_at DESC LIMIT 1",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture de la session");

    assert_eq!(
        enregistree.map(|reseau| reseau.ip()),
        Some("198.51.100.4".parse().expect("adresse attendue"))
    );
}
