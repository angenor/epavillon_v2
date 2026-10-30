//! Les sujets des envois de courriels (31/10) : `mailing:<id>` et
//! `event:<id>:mailings` gardés comme les routes que les pages relisent
//! (`engagement.mailing.send` sur l'édition, règle 8) ; `person:` d'autrui
//! refusé ; la mise en file d'un envoi signalée à sa fiche et à sa liste.

use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::crypto::Passwords;
use kernel::realtime::{Audience, Topic};
use kernel::testing::TestDb;
use serde_json::{json, Value};
use std::time::Duration;
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

async fn personne(base: &TestDb, adresse: &str) -> Uuid {
    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(adresse)
    .fetch_one(base.pool())
    .await
    .expect("personne");
    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(id)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("compte");
    id
}

/// Une édition, et son administratrice : `admin` sur la seule édition.
async fn edition_administree(base: &TestDb, sigle: &str, admin: Uuid) -> Uuid {
    let edition: Uuid = sqlx::query_scalar(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, has_pavilion)
           SELECT s.id, $1, 2027, jsonb_build_object('fr', $1), $1, lower($1)::platform.slug,
                  '{"fr":"Pavillon de la Francophonie."}'::jsonb,
                  'announced', 'hybrid', 'America/Belem'::platform.timezone_name,
                  now() + interval '20 days', now() + interval '32 days',
                  c.id, 'Belém', true
             FROM event.event_series s, reference.countries c
            WHERE s.code = 'cop_climate' AND c.iso3 = 'BRA'
        RETURNING id"#,
    )
    .bind(sigle)
    .fetch_one(base.pool())
    .await
    .expect("édition");
    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, 'admin', 'event', $2)",
    )
    .bind(admin)
    .bind(edition)
    .execute(base.pool())
    .await
    .expect("attribution");
    edition
}

async fn connexion(etat: &AppState, adresse: &str) -> String {
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

/// L'écoute s'ouvre en tâche de fond : on attend qu'un signal témoin arrive.
async fn ecoute_prete(base: &TestDb, etat: &AppState) {
    let temoin = format!("person:{}", Uuid::now_v7());
    let sujet = Topic::parse(&temoin).expect("sujet");
    for _ in 0..50 {
        sqlx::query("SELECT platform.signal($1)")
            .bind(&temoin)
            .execute(base.pool())
            .await
            .expect("signal témoin");
        tokio::time::sleep(Duration::from_millis(100)).await;
        if etat.realtime.version(&sujet, &Audience::All) > 0 {
            return;
        }
    }
    panic!("l'écoute de platform_signal ne s'est pas ouverte");
}

#[tokio::test]
async fn les_sujets_dun_envoi_suivent_le_perimetre() {
    let (base, etat) = monter().await;
    let awa = personne(&base, "awa.diallo@francophonie.org").await;
    let jean = personne(&base, "jean.kabore@francophonie.org").await;
    let cop31 = edition_administree(&base, "COP31", awa).await;
    let cop32 = edition_administree(&base, "COP32", jean).await;
    let cookie_awa = connexion(&etat, "awa.diallo@francophonie.org").await;
    let cookie_jean = connexion(&etat, "jean.kabore@francophonie.org").await;
    etat.realtime.ecouter(base.pool());
    ecoute_prete(&base, &etat).await;
    let app = test::init_service(api::build_app(&etat)).await;

    let cree = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/mailings")
            .insert_header(("Cookie", cookie_awa.clone()))
            .set_json(json!({
                "event_id": cop31,
                "subject": "Consignes",
                "body_html": "<p>Bonjour,</p>",
                "extra_addresses": ["karim@example.org", "fatou@example.org"],
                "cc": [],
            }))
            .to_request(),
    )
    .await;
    assert_eq!(cree.status(), StatusCode::OK, "brouillon");
    let envoi: Value = test::read_body_json(cree).await;
    let id = envoi["id"].as_str().expect("identifiant").to_owned();

    let fiche = format!("mailing:{id}");
    let liste = format!("event:{cop31}:mailings");
    let pouls = |cookie: String, sujets: String| {
        test::TestRequest::get()
            .uri(&format!("/api/realtime/pulse?topics={sujets}"))
            .insert_header(("Cookie", cookie))
            .to_request()
    };

    // L'administratrice d'une autre édition : ni la fiche, ni la liste, ni
    // la personne d'autrui ; sa propre liste, oui.
    let sienne = format!("event:{cop32}:mailings");
    let lu: Value = test::call_and_read_body_json(
        &app,
        pouls(
            cookie_jean.clone(),
            format!("{fiche},{liste},person:{awa},{sienne}"),
        ),
    )
    .await;
    let refuses: Vec<&str> = lu["denied"]
        .as_array()
        .expect("refus")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(refuses.contains(&fiche.as_str()), "{lu}");
    assert!(refuses.contains(&liste.as_str()), "{lu}");
    assert!(refuses.contains(&format!("person:{awa}").as_str()), "{lu}");
    assert!(lu["versions"].get(&sienne).is_some(), "{lu}");

    // Ligne ouverte sur la seule fiche d'autrui : rien d'accordé, 403.
    let ligne = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics={fiche}"))
            .insert_header(("Cookie", cookie_jean))
            .to_request(),
    )
    .await;
    assert_eq!(ligne.status(), StatusCode::FORBIDDEN);

    // L'autrice : tout accordé.
    let lu: Value = test::call_and_read_body_json(
        &app,
        pouls(cookie_awa.clone(), format!("{fiche},{liste},person:{awa}")),
    )
    .await;
    assert_eq!(lu["denied"], json!([]), "{lu}");
    assert_eq!(lu["versions"][&fiche], 0);

    // La mise en file se signale à la fiche et à la liste.
    let parti = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/mailings/{id}/send"))
            .insert_header(("Cookie", cookie_awa))
            .to_request(),
    )
    .await;
    assert_eq!(parti.status(), StatusCode::OK, "envoi");
    let sujet_fiche = Topic::parse(&fiche).expect("sujet");
    let sujet_liste = Topic::parse(&liste).expect("sujet");
    for _ in 0..30 {
        if etat.realtime.version(&sujet_fiche, &Audience::All) > 0
            && etat.realtime.version(&sujet_liste, &Audience::All) > 0
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("la mise en file n'a pas été signalée");
}
