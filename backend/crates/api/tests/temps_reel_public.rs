//! Temps réel des pages publiques et du poste de direct, sur base réelle :
//! séance passée en direct par sa réunion Zoom, séance publiée déplacée,
//! message d'incident publié, questions du public, salle Zoom ; pouls anonyme
//! qui change de version ; sujet du poste refusé sans `live.meeting.manage`.

use actix_web::http::{header, StatusCode};
use actix_web::test;
use api::state::AppState;
use kernel::context::RequestContext;
use kernel::crypto::Passwords;
use kernel::events::{EventConsumer, OutboxEvent};
use kernel::realtime::{Audience, Topic};
use kernel::testing::TestDb;
use serde_json::{json, Value};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

const MOT_DE_PASSE: &str = "Belem2027!";

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    etat.realtime.ecouter(base.pool());
    ecoute_prete(&etat).await;
    (base, etat)
}

/// L'écoute s'ouvre en tâche de fond : on attend qu'un signal témoin arrive.
async fn ecoute_prete(etat: &AppState) {
    let temoin = format!("person:{}", Uuid::now_v7());
    let sujet = Topic::parse(&temoin).expect("sujet");
    for _ in 0..50 {
        let mut tx = etat
            .db
            .write(&RequestContext::background("test-temps-reel-public"))
            .await
            .expect("transaction");
        kernel::realtime::signal(&mut tx, &temoin, None)
            .await
            .expect("signal");
        tx.commit().await.expect("commit");
        tokio::time::sleep(Duration::from_millis(100)).await;
        if etat.realtime.version(&sujet, &Audience::All) > 0 {
            return;
        }
    }
    panic!("l'écoute de platform_signal ne s'est pas ouverte");
}

fn version(etat: &AppState, sujet: &str) -> u64 {
    etat.realtime
        .version(&Topic::parse(sujet).expect("sujet"), &Audience::All)
}

/// Attend que la version du sujet dépasse `depuis` (deux secondes au plus).
async fn signale(etat: &AppState, sujet: &str, depuis: u64) -> bool {
    for _ in 0..40 {
        if version(etat, sujet) > depuis {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// Un signal témoin passé après l'écriture : s'il est arrivé et pas le sujet,
/// le sujet n'a pas été signalé (NOTIFY est livré dans l'ordre des commits).
async fn rien_signale(etat: &AppState, base: &TestDb, sujet: &str, depuis: u64) -> bool {
    let temoin = format!("person:{}", Uuid::now_v7());
    sqlx::query("SELECT platform.signal($1)")
        .bind(&temoin)
        .execute(base.pool())
        .await
        .expect("témoin");
    assert!(signale(etat, &temoin, 0).await, "témoin non reçu");
    version(etat, sujet) == depuis
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

async fn administrer(base: &TestDb, personne: Uuid, role: &str, edition: Uuid) {
    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, $2, 'event', $3)",
    )
    .bind(personne)
    .bind(role)
    .bind(edition)
    .execute(base.pool())
    .await
    .expect("attribution");
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

async fn edition(base: &TestDb, slug: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_label, edition_year, title, slug, description, status,
                participation_mode, timezone, starts_at, ends_at, has_pavilion,
                country_id, city)
           VALUES ('COP31', 2027, '{"fr":"COP31"}'::jsonb, $1::text::platform.slug,
                   '{"fr":"Pavillon."}'::jsonb, 'announced', 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-20 18:00' AT TIME ZONE 'America/Belem', true,
                   (SELECT id FROM reference.countries WHERE iso3 = 'BRA'), 'Belém')
        RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(base.pool())
    .await
    .expect("édition")
}

/// Une séance de l'édition, publiée ou en préparation.
async fn seance(base: &TestDb, edition: Uuid, slug: &str, publiee: bool) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, status, format, starts_at, ends_at, timezone, published_at)
           VALUES ($1, '{"fr":"Finance climat"}'::jsonb, $2::text::platform.slug,
                   CASE WHEN $3 THEN 'scheduled' ELSE 'planned' END::programme.session_status,
                   'hybrid',
                   timestamp '2027-11-10 14:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-10 15:30' AT TIME ZONE 'America/Belem',
                   'America/Belem'::platform.timezone_name,
                   CASE WHEN $3 THEN now() END)
        RETURNING id"#,
    )
    .bind(edition)
    .bind(slug)
    .bind(publiee)
    .fetch_one(base.pool())
    .await
    .expect("séance")
}

fn evenement(type_: &str, session: Uuid) -> OutboxEvent {
    OutboxEvent {
        id: Uuid::now_v7(),
        aggregate_schema: "live".into(),
        aggregate_type: "meeting".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: type_.into(),
        event_version: 1,
        payload: json!({ "session_id": session }),
        metadata: json!({}),
        correlation_id: None,
        occurred_at: OffsetDateTime::now_utc(),
    }
}

async fn consommer(etat: &AppState, type_: &str, session: Uuid) {
    let mut tx = etat
        .db
        .write(&RequestContext::background("test-direct"))
        .await
        .expect("transaction");
    programme::consumers::direct::DirectConsumer
        .handle(&mut tx, &evenement(type_, session))
        .await
        .expect("consommateur du direct");
    tx.commit().await.expect("commit");
}

#[tokio::test]
async fn la_seance_qui_passe_en_direct_allume_les_pages_publiques() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-direct-public").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let direct = format!("edition:{cop}:public:live");

    consommer(&etat, "live.meeting.started", publiee).await;
    assert!(
        signale(&etat, &direct, 0).await,
        "direct allumé non signalé"
    );

    let allume = version(&etat, &direct);
    consommer(&etat, "live.meeting.ended", publiee).await;
    assert!(
        signale(&etat, &direct, allume).await,
        "direct éteint non signalé"
    );

    // Une répétition sur une séance non publiée ne réveille aucun visiteur.
    let cachee = seance(&base, cop, "repetition", false).await;
    sqlx::query("UPDATE programme.sessions SET status = 'live' WHERE id = $1")
        .bind(cachee)
        .execute(base.pool())
        .await
        .expect("séance cachée en direct");
    let vu = version(&etat, &direct);
    assert!(rien_signale(&etat, &base, &direct, vu).await);
}

#[tokio::test]
async fn deplacer_une_seance_publiee_signale_le_programme_public() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-programme-public").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let cachee = seance(&base, cop, "en-preparation", false).await;
    let programme = format!("edition:{cop}:public:schedule");
    let depart = version(&etat, &programme);

    let deplacer = "UPDATE programme.sessions
                       SET starts_at = starts_at + interval '1 hour',
                           ends_at = ends_at + interval '1 hour'
                     WHERE id = $1";
    sqlx::query(deplacer)
        .bind(cachee)
        .execute(base.pool())
        .await
        .expect("séance cachée déplacée");
    assert!(
        rien_signale(&etat, &base, &programme, depart).await,
        "une séance non publiée ne concerne pas le public"
    );

    sqlx::query(deplacer)
        .bind(publiee)
        .execute(base.pool())
        .await
        .expect("séance publiée déplacée");
    assert!(
        signale(&etat, &programme, depart).await,
        "déplacement non signalé"
    );

    // Une colonne que le public ne voit pas ne signale rien.
    let vu = version(&etat, &programme);
    sqlx::query("UPDATE programme.sessions SET view_count = view_count + 1 WHERE id = $1")
        .bind(publiee)
        .execute(base.pool())
        .await
        .expect("compteur de vues");
    assert!(rien_signale(&etat, &base, &programme, vu).await);
}

#[tokio::test]
async fn le_pouls_anonyme_change_de_version_quand_le_programme_bouge() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-pouls-public").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let programme = format!("edition:{cop}:public:schedule");
    let app = test::init_service(api::build_app(&etat)).await;
    let uri = format!("/api/realtime/pulse?topics={programme}");

    let premier = test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
    assert_eq!(premier.status(), StatusCode::OK);
    let etiquette = premier
        .headers()
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_owned();
    let avant: Value = test::read_body_json(premier).await;
    let v0 = avant["versions"][&programme].as_u64().expect("version");

    sqlx::query("UPDATE programme.sessions SET title = '{\"fr\":\"Finance climat, report\"}'::jsonb WHERE id = $1")
        .bind(publiee)
        .execute(base.pool())
        .await
        .expect("titre changé");
    assert!(signale(&etat, &programme, v0).await);

    let second = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&uri)
            .insert_header((header::IF_NONE_MATCH, etiquette))
            .to_request(),
    )
    .await;
    assert_eq!(
        second.status(),
        StatusCode::OK,
        "l'ancienne empreinte ne vaut plus"
    );
    let apres: Value = test::read_body_json(second).await;
    assert!(apres["versions"][&programme].as_u64().unwrap() > v0);
}

#[tokio::test]
async fn un_incident_publie_signale_ses_editions_et_prevoit_sa_fin() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-incidents").await;
    let autre = edition(&base, "cop32-incidents").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let incidents = format!("edition:{cop}:public:incidents");
    let ailleurs = format!("edition:{autre}:public:incidents");

    let incident: Uuid = sqlx::query_scalar(
        r#"INSERT INTO live.incidents (scope, session_id, message, display_until)
           VALUES ('session', $1, '{"fr":"Vingt minutes de retard.","en":"Twenty minutes late."}'::jsonb,
                   now() + interval '2 hours')
        RETURNING id"#,
    )
    .bind(publiee)
    .fetch_one(base.pool())
    .await
    .expect("brouillon");
    // Un brouillon ne s'affiche pas : rien à signaler.
    assert!(rien_signale(&etat, &base, &incidents, 0).await);

    let mut tx = etat
        .db
        .write(&RequestContext::background("test-incident"))
        .await
        .expect("transaction");
    sqlx::query("SELECT live.publish_incident($1)")
        .bind(incident)
        .execute(&mut *tx)
        .await
        .expect("publication");
    tx.commit().await.expect("commit");
    assert!(
        signale(&etat, &incidents, 0).await,
        "publication non signalée"
    );
    assert_eq!(
        version(&etat, &ailleurs),
        0,
        "l'autre édition n'est pas concernée"
    );

    let fin: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.jobs
          WHERE task = 'live.incident_window' AND queue = 'live_incidents'
            AND payload->>'incident_id' = $1::text",
    )
    .bind(incident)
    .fetch_one(base.pool())
    .await
    .expect("travaux");
    assert_eq!(fin, 1, "la fin d'affichage est prévue");

    // Le travail de la fenêtre rejoue le signal.
    let vu = version(&etat, &incidents);
    sqlx::query("SELECT live.signal_incident($1)")
        .bind(incident)
        .execute(base.pool())
        .await
        .expect("fenêtre");
    assert!(signale(&etat, &incidents, vu).await);

    // Le retrait aussi.
    let vu = version(&etat, &incidents);
    sqlx::query("SELECT live.unpublish_incident($1, 'Réglé')")
        .bind(incident)
        .execute(base.pool())
        .await
        .expect("retrait");
    assert!(signale(&etat, &incidents, vu).await, "retrait non signalé");

    // Un message global concerne toutes les éditions.
    let vu = version(&etat, &ailleurs);
    sqlx::query(
        r#"WITH m AS (
               INSERT INTO live.incidents (scope, message)
               VALUES ('global', '{"fr":"Maintenance.","en":"Maintenance."}'::jsonb)
               RETURNING id)
           SELECT live.publish_incident(id) FROM m"#,
    )
    .execute(base.pool())
    .await
    .expect("message global");
    assert!(
        signale(&etat, &ailleurs, vu).await,
        "message global non signalé"
    );
}

#[tokio::test]
async fn les_questions_du_public_sont_signalees() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-questions").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let questions = format!("edition:{cop}:public:questions");

    sqlx::query(
        "INSERT INTO programme.session_questions (session_id, body) VALUES ($1, 'Quel financement ?')",
    )
    .bind(publiee)
    .execute(base.pool())
    .await
    .expect("question");
    assert!(signale(&etat, &questions, 0).await, "question non signalée");

    let vu = version(&etat, &questions);
    sqlx::query("UPDATE programme.session_questions SET is_visible = false WHERE session_id = $1")
        .bind(publiee)
        .execute(base.pool())
        .await
        .expect("modération");
    assert!(
        signale(&etat, &questions, vu).await,
        "modération non signalée"
    );
}

#[tokio::test]
async fn le_poste_de_direct_est_signale_et_refuse_sans_permission() {
    let (base, etat) = monter().await;
    let cop = edition(&base, "cop31-poste").await;
    let autre = edition(&base, "cop32-poste").await;
    let publiee = seance(&base, cop, "finance-climat", true).await;
    let poste = format!("edition:{cop}:desk");

    let jean = personne(&base, "jean.kabore@francophonie.org").await;
    administrer(&base, jean, "programmer", cop).await;
    let lea = personne(&base, "lea.martin@francophonie.org").await;
    administrer(&base, lea, "reviewer", cop).await;
    let marie = personne(&base, "marie.dupont@francophonie.org").await;
    administrer(&base, marie, "admin", autre).await;

    // Une entrée en salle Zoom.
    let reunion: Uuid = sqlx::query_scalar(
        "INSERT INTO live.meetings (session_id, event_id, sync_status, status)
         VALUES ($1, $2, 'not_required', 'draft') RETURNING id",
    )
    .bind(publiee)
    .bind(cop)
    .fetch_one(base.pool())
    .await
    .expect("réunion");
    let vu = version(&etat, &poste);
    sqlx::query(
        "INSERT INTO live.meeting_presences (meeting_id, participant_key, last_joined_at)
         VALUES ($1, 'p1', now())",
    )
    .bind(reunion)
    .execute(base.pool())
    .await
    .expect("présence");
    assert!(
        signale(&etat, &poste, vu).await,
        "entrée en salle non signalée"
    );

    let app = test::init_service(api::build_app(&etat)).await;
    let uri = format!("/api/realtime/pulse?topics={poste}");

    let anonyme = test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
    assert_eq!(anonyme.status(), StatusCode::UNAUTHORIZED);

    for (adresse, accorde) in [
        ("jean.kabore@francophonie.org", true),
        ("lea.martin@francophonie.org", false),
        ("marie.dupont@francophonie.org", false),
    ] {
        let cookie = connexion(&etat, adresse).await;
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Cookie", cookie))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK);
        let lu: Value = test::read_body_json(reponse).await;
        let refuse = lu["denied"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == poste.as_str());
        assert_eq!(!refuse, accorde, "{adresse} : {lu}");
    }
}
