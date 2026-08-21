//! **Les dix-sept routes de B5 sont réellement atteignables.**
//!
//! C'est le seul contrôle qui voit une route **écrite mais non montée**, et il a
//! déjà attrapé ce défaut deux fois — trois routes muettes sur vingt et une en
//! B2, puis en B4. Le risque est réel ici : `/admin/planner` est un préfixe
//! **partagé** entre deux modules, et deux `web::scope` du même préfixe ne se
//! complètent pas.
//!
//! Chaque route est frappée sur la **vraie application**, intergiciels compris.
//! On ne vérifie qu'une chose : **le chemin existe**. Le comportement, lui, est
//! éprouvé par les tests du module.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
}

/// Les routes **gardées** : sans session, toutes répondent 401.
fn chemins_gardes() -> Vec<(&'static str, String)> {
    let edition = id();
    let seance = id();
    let inscription = id();

    vec![
        // US2 — l'écran, sous un préfixe PARTAGÉ avec le module Événements.
        ("GET", format!("/api/admin/planner?event_id={edition}")),
        ("GET", format!("/api/sessions?event_id={edition}")),
        ("GET", format!("/api/sessions/conflicts?event_id={edition}")),
        ("GET", format!("/api/sessions/{seance}/speakers")),
        ("GET", format!("/api/sessions/{seance}/organizations")),
        ("GET", format!("/api/sessions/{seance}/tracks")),
        // US3, US4, US5 — les trois écritures.
        ("PUT", format!("/api/sessions/{seance}/schedule")),
        ("PUT", format!("/api/sessions/{seance}/tracks")),
        ("PUT", format!("/api/sessions/{seance}/broadcast")),
        // US8 — les inscriptions gardées.
        ("GET", format!("/api/registrations?session_id={seance}")),
        ("GET", "/api/registrations/mine".to_owned()),
        ("POST", format!("/api/registrations/{inscription}/cancel")),
        ("POST", format!("/api/registrations/{inscription}/join")),
    ]
}

/// Les routes **publiques** : elles n'exigent aucune session, et un 401 y serait
/// un défaut aussi grave qu'un 404.
///
/// **Le détail d'une séance ne peut pas être frappé à vide** : sur une adresse
/// inconnue, il rend 404 — le même que celui d'une route non montée. Une vraie
/// séance publiée est donc posée en base, et c'est un 200 qu'on attend : c'est
/// le seul contrôle qui distingue les deux.
fn chemins_publics(edition: Uuid, seance: Uuid, adresse: &str) -> Vec<(&'static str, String)> {
    vec![
        ("GET", format!("/api/schedule?event_id={edition}")),
        ("GET", format!("/api/events/{edition}/sessions/{adresse}")),
        ("GET", format!("/api/sessions/{seance}/registration-form")),
        ("POST", format!("/api/sessions/{seance}/registrations")),
    ]
}

/// Une édition et une séance **publiée**, posées directement en base : ce test
/// éprouve le montage, pas la naissance des séances.
async fn une_seance_publiee(base: &TestDb) -> (Uuid, Uuid, String) {
    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_label, edition_year, title, slug, description, status,
                participation_mode, timezone, starts_at, ends_at, country_id, city)
           VALUES ('COP31', 2027, '{"fr":"COP31"}'::jsonb,
                   'cop31-montage'::platform.slug,
                   '{"fr":"Pavillon."}'::jsonb, 'announced', 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-20 18:00' AT TIME ZONE 'America/Belem',
                   (SELECT id FROM reference.countries WHERE iso3 = 'BRA'), 'Belém')
        RETURNING id"#
    )
    .fetch_one(base.pool())
    .await
    .expect("édition");

    let adresse = "atelier-de-montage";
    let seance = sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, timezone, starts_at, ends_at,
                status, published_at)
           VALUES ($1, '{"fr":"Atelier de montage"}'::jsonb,
                   $2::text::platform.slug, 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-12 14:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-12 15:30' AT TIME ZONE 'America/Belem',
                   'scheduled', now())
        RETURNING id"#,
        edition,
        adresse
    )
    .fetch_one(base.pool())
    .await
    .expect("séance publiée");

    (edition, seance, adresse.to_owned())
}

fn requete(verbe: &str) -> test::TestRequest {
    match verbe {
        "GET" => test::TestRequest::get(),
        "POST" => test::TestRequest::post(),
        "PUT" => test::TestRequest::put(),
        "DELETE" => test::TestRequest::delete(),
        autre => panic!("verbe inattendu : {autre}"),
    }
}

#[actix_web::test]
async fn les_dix_sept_routes_des_seances_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let (edition, seance, adresse) = une_seance_publiee(&base).await;

    let gardes = chemins_gardes();
    let publics = chemins_publics(edition, seance, &adresse);
    assert_eq!(
        gardes.len() + publics.len(),
        17,
        "les dix-sept routes du contrat"
    );

    for (verbe, chemin) in gardes {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée — deux scopes du même préfixe ne se \
             complètent pas, et le second est muet"
        );
        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} a répondu {} — on attendait un refus d'authentification",
            reponse.status()
        );
    }

    for (verbe, chemin) in publics {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée"
        );
        assert_ne!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} est PUBLIQUE : une programmation publiée se lit sans session"
        );
    }
}

/// 🔴 **Les deux routes de B3 sous `/admin/planner` répondent toujours.**
///
/// C'est ce test-là qui verrait une route devenue muette : le scope est composé
/// à partir de deux modules, et l'ajout du second aurait pu rendre le premier
/// inaccessible.
#[actix_web::test]
async fn les_deux_routes_de_b3_repondent_toujours() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;
    let edition = id();

    for (verbe, chemin) in [
        (
            "GET",
            format!("/api/admin/planner/readiness?event_id={edition}"),
        ),
        ("POST", "/api/admin/planner/publish".to_owned()),
    ] {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} — le contrôle préalable et la publication de B3 \
             doivent survivre à l'arrivée de B5 dans le même préfixe"
        );
    }
}

/// **Un seul chemin sert la lecture de contrôle** (écart n° 121).
///
/// Le contrat du front déclare `/sessions/publication-readiness`, que B3 sert
/// déjà sous `/admin/planner/readiness`. Livrer deux chemins pour une même
/// lecture dans deux modules différents garantit qu'ils divergeront : le second
/// n'est **pas** servi, et ce test le fige.
#[actix_web::test]
async fn le_second_chemin_de_controle_nest_pas_servi() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(
                "/api/sessions/publication-readiness?event_id=00000000-0000-0000-0000-000000000000",
            )
            .to_request(),
    )
    .await;

    assert_eq!(
        reponse.status(),
        StatusCode::NOT_FOUND,
        "un seul chemin sert cette lecture, et il appartient à B3"
    );
}

/// **Les chemins littéraux ne sont pas capturés par les chemins paramétrés.**
///
/// `/sessions/conflicts` et `/registrations/mine` ont deux segments, leurs
/// homologues paramétrés en ont trois : le risque de capture n'existe donc pas
/// ici. Le test le mesure plutôt que de le supposer — c'est la même vigilance
/// qui a manqué en B2.
#[actix_web::test]
async fn les_chemins_litteraux_ne_sont_pas_captures() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    for (verbe, chemin) in [
        (
            "GET",
            "/api/sessions/conflicts?event_id=00000000-0000-0000-0000-000000000000",
        ),
        ("GET", "/api/registrations/mine"),
    ] {
        let reponse = test::call_service(&app, requete(verbe).uri(chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} a été capturé par un chemin paramétré : un 404 le dirait"
        );
    }
}
