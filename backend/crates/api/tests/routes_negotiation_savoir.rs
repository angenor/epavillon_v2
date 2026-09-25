//! Le paquet du savoir, **à travers HTTP** : monté sous `/api`, ouvert sans
//! session, son `ETag` public qui suit la langue, le `304` — même derrière un
//! relais qui suffixe l'empreinte —, et `since` illisible. Puis les termes
//! favoris : la session exigée, leur `ETag` privé et leur `304`. Enfin les
//! retours et signalements : `401` sans session, `201` puis `200` au rejeu.

use actix_web::http::header::{ACCEPT_LANGUAGE, CACHE_CONTROL, ETAG, IF_NONE_MATCH, VARY};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::{json, Value};
use uuid::Uuid;

const PAQUET: &str = "/api/negotiation/knowledge";
const FAVORIS: &str = "/api/negotiation/me/glossary-favorites";
const MOT_DE_PASSE: &str = "Belem2027!";
const LECTRICE: &str = "lectrice@example.org";

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

#[actix_web::test]
async fn une_empreinte_suffixee_par_un_relais_rend_encore_304() {
    let base = TestDb::new().await;
    let etat = AppState::new(base.db(), kernel::testing::test_config(base.url()))
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let entier = test::call_service(&app, test::TestRequest::get().uri(PAQUET).to_request()).await;
    let empreinte = entete(&entier, ETAG);
    let nu = empreinte.trim_matches('"');

    for presentee in [format!("W/\"{nu}-br\""), format!("\"{nu}-gzip\"")] {
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(PAQUET)
                .insert_header((IF_NONE_MATCH, presentee.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::NOT_MODIFIED, "{presentee}");
        assert_eq!(entete(&reponse, ETAG), empreinte);
    }
}

async fn compte(base: &TestDb) {
    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");
    let person_id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(LECTRICE)
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
}

async fn terme_publie(base: &TestDb) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO negotiation.glossary_entries
               (slug, family_term_id, term, translation, definition, status)
           SELECT '', id, 'Contact group', '{"fr":"traduction"}', '{"fr":"définition"}', 'published'
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'glossary_family' AND code = 'meetings'
           RETURNING id"#,
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion du terme")
}

#[actix_web::test]
async fn les_termes_favoris_demandent_une_session_et_rendent_304() {
    let base = TestDb::new().await;
    compte(&base).await;
    let groupe = terme_publie(&base).await;
    let etat = AppState::new(base.db(), kernel::testing::test_config(base.url()))
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let anonyme =
        test::call_service(&app, test::TestRequest::get().uri(FAVORIS).to_request()).await;
    assert_eq!(anonyme.status(), StatusCode::UNAUTHORIZED);

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({
                "email": LECTRICE,
                "password": MOT_DE_PASSE,
                "remember_me": false,
                "client": { "kind": "app", "device_id": "9f2c-appareil", "platform": "android" },
            }))
            .to_request(),
    )
    .await;
    assert_eq!(connexion.status(), StatusCode::OK);
    let cookie = connexion
        .response()
        .cookies()
        .find(|c| c.name() == "epavillon_at")
        .map(|c| format!("{}={}", c.name(), c.value()))
        .expect("cookie d'accès");

    let pose = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("{FAVORIS}/{groupe}"))
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(pose.status(), StatusCode::NO_CONTENT);

    let liste = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(FAVORIS)
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(liste.status(), StatusCode::OK);
    assert_eq!(entete(&liste, CACHE_CONTROL), "private, no-cache");
    let empreinte = entete(&liste, ETAG);
    let corps: Value = test::read_body_json(liste).await;
    assert_eq!(corps["entry_ids"], json!([groupe]));

    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(FAVORIS)
            .insert_header(("cookie", cookie))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(entete(&inchange, ETAG), empreinte);
    assert_eq!(entete(&inchange, CACHE_CONTROL), "private, no-cache");
}

#[actix_web::test]
async fn une_lecture_de_faq_se_compte_sans_session() {
    let base = TestDb::new().await;
    let etat = AppState::new(base.db(), kernel::testing::test_config(base.url()))
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let lue = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/negotiation/faq/{}/read", Uuid::now_v7()))
            .to_request(),
    )
    .await;
    assert_eq!(
        lue.status(),
        StatusCode::NO_CONTENT,
        "inconnue : 204 quand même"
    );
}

macro_rules! connecter {
    ($app:expr) => {{
    let connexion = test::call_service(
        $app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({
                "email": LECTRICE,
                "password": MOT_DE_PASSE,
                "remember_me": false,
                "client": { "kind": "app", "device_id": "9f2c-appareil", "platform": "android" },
            }))
            .to_request(),
    )
    .await;
    assert_eq!(connexion.status(), StatusCode::OK);
    connexion
        .response()
        .cookies()
        .find(|c| c.name() == "epavillon_at")
        .map(|c| format!("{}={}", c.name(), c.value()))
        .expect("cookie d'accès")
    }};
}

#[actix_web::test]
async fn retours_et_signalements_demandent_une_session_et_se_rejouent() {
    let base = TestDb::new().await;
    compte(&base).await;
    let faq: Uuid = sqlx::query_scalar(
        r#"INSERT INTO negotiation.faq_entries
               (section_term_id, question, answer, status, verified_on, verified_by)
           SELECT t.id, '{"fr":"Qui préside ?"}', '{"fr":"La présidence."}', 'published',
                  current_date, p.id
             FROM reference.taxonomy_terms t, identity.people p
            WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
           RETURNING id"#,
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de l'entrée");
    let etat = AppState::new(base.db(), kernel::testing::test_config(base.url()))
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;
    let retour = format!("/api/negotiation/faq/{faq}/feedback");
    let signalement = format!("/api/negotiation/faq/{faq}/reports");
    let voix = "/api/negotiation/me/faq-feedback";
    let corps_signalement = json!({ "client_ref": Uuid::now_v7(), "reasons": ["wrong"] });

    for requete in [
        test::TestRequest::put()
            .uri(&retour)
            .set_json(json!({ "helpful": true })),
        test::TestRequest::post()
            .uri(&signalement)
            .set_json(corps_signalement.clone()),
        test::TestRequest::get().uri(voix),
    ] {
        let r = test::call_service(&app, requete.to_request()).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }

    let cookie = connecter!(&app);
    let vote = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&retour)
            .insert_header(("cookie", cookie.clone()))
            .set_json(json!({ "helpful": false, "missing_reason": "too_vague" }))
            .to_request(),
    )
    .await;
    assert_eq!(vote.status(), StatusCode::OK);

    let mut recus = Vec::new();
    for attendu in [StatusCode::CREATED, StatusCode::OK] {
        let r = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&signalement)
                .insert_header(("cookie", cookie.clone()))
                .set_json(corps_signalement.clone())
                .to_request(),
        )
        .await;
        assert_eq!(r.status(), attendu);
        let corps: Value = test::read_body_json(r).await;
        recus.push(corps["id"].clone());
    }
    assert_eq!(recus[0], recus[1], "le rejeu rend le même reçu");

    let lues = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(voix)
            .insert_header(("cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(lues.status(), StatusCode::OK);
    assert_eq!(entete(&lues, CACHE_CONTROL), "private, no-cache");
    let empreinte = entete(&lues, ETAG);
    let corps: Value = test::read_body_json(lues).await;
    assert_eq!(corps["feedback"][0]["missing_reason"], "too_vague");
    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(voix)
            .insert_header(("cookie", cookie))
            .insert_header((IF_NONE_MATCH, empreinte))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
}
