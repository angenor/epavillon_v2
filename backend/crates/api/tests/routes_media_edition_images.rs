//! **L'obligation que B3 avait laissée est refermée, et c'est SA route qui le
//! dit.**
//!
//! Depuis B3, le formulaire d'édition accepte trois identifiants d'image **sans
//! les poser** : le rattachement est une écriture dans `media.attachments`,
//! schéma d'un autre module. B6 pose ces rattachements — et la preuve ne peut
//! pas être une relecture du code de B6 : elle doit venir de **la lecture de
//! B3**, sur la vraie application.
//!
//! # Pourquoi ce test vit ici et pas dans le crate Média
//!
//! Frapper `GET /events/{slug}` depuis `crates/modules/media/tests/` exigerait
//! une dépendance de développement vers `api` — qui dépend de cinq crates de
//! module. Le contrôle bloquant du jalon, `cargo tree -p media` sans arête vers
//! un autre module, la verrait : `cargo tree` liste aussi les dépendances de
//! développement.
//!
//! L'autre moitié de la mesure — l'écriture en un geste, le retrait sélectif,
//! les quatre refus — vit dans `media/tests/edition_images.rs`.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use media::domain::attachment::{AttachmentAssignment, AttachmentBatch};
use media::state::MediaState;
use std::sync::Arc;
use uuid::Uuid;

/// Un objet **servable**, écrit directement : le dépôt a ses propres tests, et
/// ce qui se mesure ici est le rattachement et la lecture qui en découle.
async fn objet(base: &TestDb, proprietaire: Uuid, nom: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (object_key, checksum_sha256, mime_type, byte_size, original_filename,
                owner_person_id, status, scan_verdict, scan_engine, scanned_at, alt_text)
           VALUES ($1, encode(digest($1, 'sha256'), 'hex'), 'image/png', 4096, $2,
                   $3, 'ready', 'unsupported', 'none', now(),
                   '{"fr":"Une image d''épreuve","en":"A test image"}'::jsonb::platform.i18n_text)
        RETURNING id"#,
        format!("2026/08/{}/{nom}.png", Uuid::now_v7().simple()),
        nom,
        proprietaire
    )
    .fetch_one(base.pool())
    .await
    .expect("écriture de l'objet")
}

/// **Les trois déclinaisons posées par B6 apparaissent dans la page de B3.**
#[actix_web::test]
async fn les_trois_declinaisons_dune_edition_apparaissent_dans_la_page_de_b3() {
    let base = TestDb::new().await;
    let config = Arc::new(kernel::testing::test_config(base.url()));
    let etat = api::state::AppState::new(base.db(), (*config).clone())
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let (edition, slug, administratrice) = edition_et_son_administratrice(&base).await;

    // **Avant** : la page existe et ses trois images sont nulles. C'est le cas
    // courant, et il ne doit pas ressembler à une panne.
    let avant: serde_json::Value = {
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/events/{slug}"))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK);
        test::read_body_json(reponse).await
    };
    for role in ["banner", "cover", "thumbnail"] {
        assert!(avant[role].is_null(), "{role} devrait être vide au départ");
    }

    let bandeau = objet(&base, administratrice, "bandeau").await;
    let couverture = objet(&base, administratrice, "couverture").await;
    let vignette = objet(&base, administratrice, "vignette").await;

    let media = MediaState::new(base.db(), config);
    let ctx = kernel::context::RequestContext::new("test-b6", "fr").with_actor(administratrice);

    media::service::attach::remplacer(
        &media,
        &ctx,
        administratrice,
        &AttachmentBatch {
            owner_schema: "event".to_owned(),
            owner_table: "events".to_owned(),
            owner_id: edition,
            assignments: vec![
                affectation("banner", bandeau),
                affectation("cover", couverture),
                affectation("thumbnail", vignette),
            ],
        },
    )
    .await
    .expect("les trois déclinaisons en un geste");

    let apres: serde_json::Value = {
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/events/{slug}"))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK);
        test::read_body_json(reponse).await
    };
    for (role, attendu) in [
        ("banner", bandeau),
        ("cover", couverture),
        ("thumbnail", vignette),
    ] {
        let image = &apres[role];
        assert!(!image.is_null(), "{role} devrait être résolue par B3");
        assert_eq!(image["asset_id"], serde_json::json!(attendu), "{role}");
        // **La base ne stocke jamais d'URL** : celle-ci est composée à la
        // lecture, et aucune clé nue ne sort de l'API.
        assert!(
            image["url"].as_str().expect("adresse").starts_with("http"),
            "{role} doit porter une adresse composée"
        );
        assert!(!image["alt_text"].is_null(), "{role} doit porter son texte");
    }
}

fn affectation(role: &str, asset_id: Uuid) -> AttachmentAssignment {
    AttachmentAssignment {
        role: role.to_owned(),
        asset_id: Some(asset_id),
        alt_text_override: None,
    }
}

/// Une édition annoncée — donc publique — et une personne qui l'administre,
/// **elle et aucune autre**.
async fn edition_et_son_administratrice(base: &TestDb) -> (Uuid, String, Uuid) {
    let serie = sqlx::query_scalar!("SELECT id FROM event.event_series WHERE code = 'cop_climate'")
        .fetch_one(base.pool())
        .await
        .expect("série climat du semis");

    let edition = sqlx::query!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, has_pavilion)
           VALUES ($1, 'COP31', 2027,
                   '{"fr":"COP31 — Conférence des Parties","en":"COP31"}'::jsonb,
                   'COP31', 'cop31-belem'::platform.slug,
                   '{"fr":"Pavillon de la Francophonie.","en":"Francophonie pavilion."}'::jsonb,
                   'announced', 'online', 'America/Belem'::platform.timezone_name,
                   now() + interval '1 year', now() + interval '1 year 10 days', true)
        RETURNING id, slug::text AS "slug!""#,
        serie
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de l'édition");

    let administratrice = sqlx::query_scalar!(
        r#"INSERT INTO identity.people
               (primary_email, first_name, last_name, email_verified_at, status)
           VALUES ('admin@ifdd.org'::text::platform.email, 'Sylvie', 'Nomo', now(), 'active')
        RETURNING id"#
    )
    .fetch_one(base.pool())
    .await
    .expect("insertion de la personne");

    sqlx::query!(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, 'admin', 'event', $2)",
        administratrice,
        edition.id
    )
    .execute(base.pool())
    .await
    .expect("attribution du rôle");

    (edition.id, edition.slug, administratrice)
}
