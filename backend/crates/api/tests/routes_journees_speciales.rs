//! **Les routes de la page d'un fil (30/10) sont montées** sur la vraie
//! application : huit sous le scope `/admin/tracks`, une sous le préfixe
//! partagé `/admin/planner`, une publique. Sans session, une route gardée rend
//! 401, jamais 404 ; la page publique rend 200 pour un fil publié et 404 pour
//! un fil qui ne l'est pas.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
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
async fn les_routes_de_la_page_dun_fil_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_label, edition_year, title, slug, description, status,
                participation_mode, timezone, starts_at, ends_at, country_id, city, has_pavilion)
           VALUES ('CdP31', 2026, '{"fr":"CdP31"}'::jsonb, 'cdp31-montage'::platform.slug,
                   '{"fr":"Pavillon."}'::jsonb, 'announced', 'hybrid',
                   'Europe/Istanbul'::platform.timezone_name,
                   timestamp '2026-11-09 09:00' AT TIME ZONE 'Europe/Istanbul',
                   timestamp '2026-11-20 18:00' AT TIME ZONE 'Europe/Istanbul',
                   (SELECT id FROM reference.countries WHERE iso3 = 'TUR'), 'Antalya', true)
        RETURNING id"#
    )
    .fetch_one(base.pool())
    .await
    .expect("édition");

    for (slug, publie) in [("journee-finance", true), ("journee-cachee", false)] {
        sqlx::query!(
            r#"INSERT INTO event.programme_tracks (event_id, code, slug, kind, title, published_at)
               VALUES ($1, replace($2, '-', '_'), $2::text::platform.slug, 'special_day',
                       '{"fr":"Journée"}'::jsonb, CASE WHEN $3 THEN now() END)"#,
            edition,
            slug,
            publie
        )
        .execute(base.pool())
        .await
        .expect("fil");
    }

    let (fil, partie, moment, seance) = (id(), id(), id(), id());
    for (verbe, chemin) in [
        ("GET", format!("/api/admin/tracks/{fil}/page")),
        ("PUT", format!("/api/admin/tracks/{fil}/page")),
        ("POST", format!("/api/admin/tracks/{fil}/parts")),
        ("PUT", format!("/api/admin/tracks/{fil}/parts/{partie}")),
        ("DELETE", format!("/api/admin/tracks/{fil}/parts/{partie}")),
        ("POST", format!("/api/admin/tracks/{fil}/moments")),
        ("PUT", format!("/api/admin/tracks/{fil}/moments/{moment}")),
        ("DELETE", format!("/api/admin/tracks/{fil}/moments/{moment}")),
        ("PUT", format!("/api/admin/planner/tracks/{fil}/sessions/{seance}")),
    ] {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;
        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} devrait exiger une session, pas disparaître"
        );
    }

    let publiee = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/events/cdp31-montage/tracks/journee-finance")
            .to_request(),
    )
    .await;
    assert_eq!(publiee.status(), StatusCode::OK, "page publique sans session");

    let cachee = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/events/cdp31-montage/tracks/journee-cachee")
            .to_request(),
    )
    .await;
    assert_eq!(cachee.status(), StatusCode::NOT_FOUND, "fil non publié");
}
