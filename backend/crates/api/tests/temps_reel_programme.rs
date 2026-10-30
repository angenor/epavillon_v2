//! Temps réel du module Programme, sur base réelle : échanges d'un dossier
//! (audience du comité), état du dossier et grille du planificateur signalés
//! au commit, sujets d'autrui refusés.

use actix_web::body::MessageBody;
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::context::RequestContext;
use kernel::crypto::Passwords;
use kernel::realtime::{Audience, Topic};
use kernel::testing::TestDb;
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;

const MOT_DE_PASSE: &str = "Belem2027!";

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    etat.realtime.ecouter(base.pool());
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

async fn organisation(base: &TestDb, nom: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO org.organizations
             (legal_name, slug, organization_type_code, status, verified_at)
         VALUES ($1, platform.slugify($1)::platform.slug, 'ngo_association', 'active', now())
         RETURNING id",
    )
    .bind(nom)
    .fetch_one(base.pool())
    .await
    .expect("organisation")
}

async fn adherer(base: &TestDb, organisation: Uuid, personne: Uuid) {
    sqlx::query(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, approved_at)
         VALUES ($1, $2, 'member', 'active', 'Chargée de projet', now())",
    )
    .bind(organisation)
    .bind(personne)
    .execute(base.pool())
    .await
    .expect("adhésion");
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

async fn appel_ouvert(base: &TestDb, edition: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at)
           VALUES ($1, 'principal', '{"fr":"Appel"}'::jsonb, 'open',
                   now() - interval '1 day', now() + interval '30 days')
        RETURNING id"#,
    )
    .bind(edition)
    .fetch_one(base.pool())
    .await
    .expect("appel")
}

async fn dossier(base: &TestDb, appel: Uuid, edition: Uuid, org: Uuid, par: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by, title, slug,
                objectives, detailed_presentation, format)
           VALUES ($1, $2, $3, $4, '{"fr":"Finance durable"}'::jsonb,
                   ('dossier-' || substr(md5(random()::text), 1, 8))::platform.slug,
                   '{"fr":"Objectifs."}'::jsonb, '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'hybrid')
        RETURNING id"#,
    )
    .bind(appel)
    .bind(edition)
    .bind(org)
    .bind(par)
    .fetch_one(base.pool())
    .await
    .expect("dossier")
}

struct Terrain {
    edition: Uuid,
    autre_edition: Uuid,
    organisation: Uuid,
    autre_organisation: Uuid,
    dossier: Uuid,
    dossier_dautrui: Uuid,
}

/// La déposante (`awa…`) d'une organisation, une autre organisation, un
/// administrateur de la COP31 (`jean…`) et un de l'autre édition (`marie…`).
async fn terrain(base: &TestDb) -> Terrain {
    let edition_ = edition(base, "cop31-temps-reel").await;
    let autre_edition = edition(base, "cop32-temps-reel").await;
    let appel = appel_ouvert(base, edition_).await;
    let org = organisation(base, "Réseau Climat Sahel").await;
    let autre_org = organisation(base, "Alliance Océan Indien").await;

    let awa = personne(base, "awa.diallo@example.org").await;
    adherer(base, org, awa).await;
    let fatou = personne(base, "fatou.sow@example.org").await;
    adherer(base, autre_org, fatou).await;
    let jean = personne(base, "jean.kabore@francophonie.org").await;
    administrer(base, jean, "admin", edition_).await;
    let marie = personne(base, "marie.dupont@francophonie.org").await;
    administrer(base, marie, "admin", autre_edition).await;

    Terrain {
        edition: edition_,
        autre_edition,
        organisation: org,
        autre_organisation: autre_org,
        dossier: dossier(base, appel, edition_, org, awa).await,
        dossier_dautrui: dossier(base, appel, edition_, autre_org, fatou).await,
    }
}

async fn connexion(etat: &AppState, adresse: &str) -> String {
    let app = test::init_service(api::build_app(etat)).await;
    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": adresse, "password": MOT_DE_PASSE }))
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

async fn morceau<B: MessageBody>(corps: &mut Pin<Box<B>>, delai: Duration) -> Option<String> {
    let lu = tokio::time::timeout(
        delai,
        std::future::poll_fn(|cx| corps.as_mut().poll_next(cx)),
    )
    .await
    .ok()??;
    Some(String::from_utf8_lossy(&lu.ok()?).into_owned())
}

async fn attendre<B: MessageBody>(
    corps: &mut Pin<Box<B>>,
    nom: &str,
    delai: Duration,
) -> Option<String> {
    let fin = tokio::time::Instant::now() + delai;
    while let Some(m) = morceau(corps, fin - tokio::time::Instant::now()).await {
        if m.contains(&format!("event: {nom}\n")) {
            return Some(m);
        }
    }
    None
}

/// L'écoute s'ouvre en tâche de fond : on attend qu'un signal témoin arrive.
async fn ecoute_prete(etat: &AppState) {
    let temoin = format!("person:{}", Uuid::now_v7());
    let sujet = Topic::parse(&temoin).expect("sujet");
    for _ in 0..50 {
        let mut tx = etat
            .db
            .write(&RequestContext::background("test-temps-reel"))
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

fn deposant() -> Audience {
    Audience::Only(vec!["submitter".into()])
}

fn version(etat: &AppState, sujet: &str, audience: &Audience) -> u64 {
    etat.realtime
        .version(&Topic::parse(sujet).expect("sujet"), audience)
}

/// La version d'un sujet a dépassé `avant` dans les deux secondes.
async fn signale(etat: &AppState, sujet: &str, audience: &Audience, avant: u64) -> bool {
    for _ in 0..20 {
        if version(etat, sujet, audience) > avant {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn un_echange_du_comite_n_atteint_pas_le_deposant() {
    let (base, etat) = monter().await;
    let t = terrain(&base).await;
    ecoute_prete(&etat).await;
    let awa = connexion(&etat, "awa.diallo@example.org").await;
    let jean = connexion(&etat, "jean.kabore@francophonie.org").await;
    let app = test::init_service(api::build_app(&etat)).await;

    let dossier = format!("proposal:{}", t.dossier);
    let espace = format!("organization:{}", t.organisation);
    let flux = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics={dossier},{espace}"))
            .insert_header(("Cookie", awa.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(flux.status(), StatusCode::OK);
    let mut corps = Box::pin(flux.into_body());
    let pret = attendre(&mut corps, "ready", Duration::from_secs(2))
        .await
        .expect("ready");
    assert!(
        pret.contains("\"denied\":[]"),
        "deux sujets accordés : {pret}"
    );

    let ecrire = |visibilite: &'static str, cookie: String| {
        test::TestRequest::post()
            .uri(&format!("/api/proposals/{}/comments", t.dossier))
            .insert_header(("Cookie", cookie))
            .set_json(
                serde_json::json!({ "body": "Merci pour le dossier.", "visibility": visibilite }),
            )
            .to_request()
    };

    let liste = format!("event:{}:proposals", t.edition);
    let (fiche_avant, espace_avant, liste_avant) = (
        version(&etat, &dossier, &deposant()),
        version(&etat, &espace, &deposant()),
        version(&etat, &liste, &Audience::All),
    );

    // Délibération du comité : signalée au comité, jamais au déposant.
    let note = test::call_service(&app, ecrire("committee", jean.clone())).await;
    assert!(note.status().is_success(), "{}", note.status());
    assert!(signale(&etat, &dossier, &Audience::All, 0).await);
    assert!(signale(&etat, &liste, &Audience::All, liste_avant).await);
    assert_eq!(version(&etat, &dossier, &deposant()), fiche_avant);
    assert_eq!(version(&etat, &espace, &deposant()), espace_avant);
    assert!(attendre(&mut corps, "signal", Duration::from_secs(1))
        .await
        .is_none());

    // Message au déposant : la fiche et l'espace de l'organisation suivent.
    let partage = test::call_service(&app, ecrire("submitter", jean)).await;
    assert!(partage.status().is_success());
    let recu = attendre(&mut corps, "signal", Duration::from_secs(3))
        .await
        .expect("signal livré au déposant");
    assert!(recu.contains(&dossier) || recu.contains(&espace), "{recu}");
    assert!(signale(&etat, &dossier, &deposant(), fiche_avant).await);
    assert!(signale(&etat, &espace, &deposant(), espace_avant).await);
}

#[tokio::test]
async fn l_etat_du_dossier_est_signale_au_commit() {
    let (base, etat) = monter().await;
    let t = terrain(&base).await;
    ecoute_prete(&etat).await;
    let statut = format!("proposal:{}:status", t.dossier);
    let espace = format!("organization:{}", t.organisation);
    let liste = format!("event:{}:proposals", t.edition);
    let avant = [
        version(&etat, &statut, &deposant()),
        version(&etat, &espace, &deposant()),
        version(&etat, &liste, &Audience::All),
    ];

    let transition = |valider: bool| {
        let etat = &etat;
        async move {
            let mut tx = etat
                .db
                .write(&RequestContext::background("test-etat"))
                .await
                .expect("transaction");
            sqlx::query(
                "UPDATE programme.proposals SET status = 'withdrawn', submitted_at = now()
                  WHERE id = $1",
            )
            .bind(t.dossier)
            .execute(&mut *tx)
            .await
            .expect("retrait");
            if valider {
                tx.commit().await.expect("commit");
            } else {
                tx.rollback().await.expect("rollback");
            }
        }
    };

    transition(false).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        version(&etat, &statut, &deposant()),
        avant[0],
        "rollback muet"
    );

    transition(true).await;
    assert!(
        signale(&etat, &statut, &deposant(), avant[0]).await,
        "visible du déposant"
    );
    assert!(signale(&etat, &espace, &deposant(), avant[1]).await);
    assert!(signale(&etat, &liste, &Audience::All, avant[2]).await);
}

#[tokio::test]
async fn les_sujets_d_une_autre_organisation_sont_refuses() {
    let (base, etat) = monter().await;
    let t = terrain(&base).await;
    let awa = connexion(&etat, "awa.diallo@example.org").await;
    let app = test::init_service(api::build_app(&etat)).await;

    for sujet in [
        format!("organization:{}", t.autre_organisation),
        format!("proposal:{}", t.dossier_dautrui),
        format!("proposal:{}:status", t.dossier_dautrui),
        format!("event:{}:proposals", t.edition),
        format!("event:{}:planner", t.edition),
    ] {
        let refus = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/realtime/stream?topics={sujet}"))
                .insert_header(("Cookie", awa.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(refus.status(), StatusCode::FORBIDDEN, "{sujet}");
    }

    let sien = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/realtime/pulse?topics=proposal:{},organization:{}",
                t.dossier, t.organisation
            ))
            .insert_header(("Cookie", awa))
            .to_request(),
    )
    .await;
    assert_eq!(sien.status(), StatusCode::OK);
    let lu: serde_json::Value = test::read_body_json(sien).await;
    assert_eq!(lu["denied"].as_array().map(Vec::len), Some(0), "{lu}");
}

#[tokio::test]
async fn le_planificateur_est_refuse_hors_perimetre_et_signale_au_commit() {
    let (base, etat) = monter().await;
    let t = terrain(&base).await;
    ecoute_prete(&etat).await;
    let jean = connexion(&etat, "jean.kabore@francophonie.org").await;
    let marie = connexion(&etat, "marie.dupont@francophonie.org").await;
    let app = test::init_service(api::build_app(&etat)).await;
    let grille = format!("event:{}:planner", t.edition);

    // Administratrice d'une autre édition : refusé (règle 8).
    let hors = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics={grille}"))
            .insert_header(("Cookie", marie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(hors.status(), StatusCode::FORBIDDEN);
    let sienne = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/realtime/stream?topics=event:{}:planner",
                t.autre_edition
            ))
            .insert_header(("Cookie", marie))
            .to_request(),
    )
    .await;
    assert_eq!(sienne.status(), StatusCode::OK);

    let flux = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics={grille}"))
            .insert_header(("Cookie", jean))
            .to_request(),
    )
    .await;
    assert_eq!(flux.status(), StatusCode::OK);
    let mut corps = Box::pin(flux.into_body());
    attendre(&mut corps, "ready", Duration::from_secs(2))
        .await
        .expect("ready");

    // Une séance placée par un collègue : la grille est signalée au commit.
    sqlx::query(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, timezone, starts_at, ends_at, status)
           VALUES ($1, '{"fr":"Séance"}'::jsonb, 'seance-temps-reel'::platform.slug, 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-12 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-12 10:00' AT TIME ZONE 'America/Belem', 'planned')"#,
    )
    .bind(t.edition)
    .execute(base.pool())
    .await
    .expect("séance");
    let recu = attendre(&mut corps, "signal", Duration::from_secs(3))
        .await
        .expect("signal du planificateur");
    assert!(recu.contains(&grille), "{recu}");
}
