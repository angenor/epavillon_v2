//! Les cinq routes de l'import et de l'ordre du jour exigent
//! `negotiation.space.manage` **sur la portée globale** (FR-040). Le rôle
//! `admin` porte cette permission sur un événement : l'administrateur de la
//! COP31 est refusé jusque sur l'import de sa COP, avant toute écriture.

mod commun;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::{Method, StatusCode};
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use commun::documents::administratrice;
use commun::{attribuer, personne, Bac};
use kernel::auth::Scope;
use kernel::context::RequestContext;
use negotiation::domain::permissions::SPACE_MANAGE;
use serde_json::{json, Value};
use uuid::Uuid;

const ACTEUR: &str = "x-essai-acteur";

macro_rules! back_office {
    ($bac:expr) => {
        init_service(
            App::new()
                .app_data(web::Data::new($bac.db()))
                .app_data(web::Data::new($bac.state.clone()))
                .wrap_fn(|req, srv| {
                    let acteur = req
                        .headers()
                        .get(ACTEUR)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| Uuid::parse_str(v).ok());
                    let ctx = RequestContext::new(RequestContext::generated_request_id(), "fr");
                    req.extensions_mut().insert(match acteur {
                        Some(a) => ctx.with_actor(a),
                        None => ctx,
                    });
                    srv.call(req)
                })
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

async fn frapper<S, R, B>(app: &S, requete: R) -> (StatusCode, Value)
where
    S: Service<R, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let reponse = call_service(app, requete).await;
    let statut = reponse.status();
    let octets = read_body(reponse).await;
    (
        statut,
        serde_json::from_slice(&octets).unwrap_or(Value::Null),
    )
}

fn appel(verbe: &str, uri: &str, acteur: Uuid, corps: Option<Value>) -> TestRequest {
    let methode = Method::from_bytes(verbe.as_bytes()).expect("verbe HTTP");
    let r = TestRequest::default()
        .method(methode)
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()));
    match corps {
        Some(c) => r.set_json(c),
        None => r,
    }
}

fn reglage() -> Value {
    json!({
        "enabled": true, "reader": "archive", "archive_name": "cop30/lecture-1",
        "archive_first_day": "2026-11-09", "live_url": null, "time_correction_minutes": 60,
        "official_programme_url": "https://unfccc.int/cop31/schedule",
        "interval_seconds": 300, "missed_threshold": 3
    })
}

/// Travaux, imports, points rattachés, traces : ce qu'une écriture laisserait.
async fn ecritures(bac: &Bac) -> (i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM platform.jobs),
                (SELECT count(*) FROM negotiation.official_imports),
                (SELECT count(*) FROM negotiation.agenda_items WHERE theme_term_id IS NOT NULL),
                (SELECT count(*) FROM platform.audit_log)",
    )
    .fetch_one(bac.pool())
    .await
    .expect("compteurs")
}

#[tokio::test]
async fn ladministrateur_dune_edition_est_refuse_sur_chaque_route_de_limport() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let admin_cop = personne(&bac, "admin.cop31@example.org").await;
    let slug = "cop31-perimetre-import";
    let edition: Uuid = sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
           VALUES (2026, '{"fr":"COP31"}'::jsonb, $1::text::platform.slug, '{"fr":"COP31"}'::jsonb,
                   'online', 'Asia/Istanbul', now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("édition");
    attribuer(&bac, admin_cop, "admin", "event", Some(edition)).await;
    assert!(
        kernel::auth::has_permission(bac.pool(), admin_cop, SPACE_MANAGE, Scope::Event(edition))
            .await
            .expect("permission"),
        "sur son édition, la permission existe : c'est le piège"
    );

    let point: Uuid = sqlx::query_scalar(
        "INSERT INTO negotiation.agenda_items (event_id, code, title, first_read_at)
         VALUES ($1, 'SBI 12', 'Gender and climate change', now()) RETURNING id",
    )
    .bind(edition)
    .fetch_one(bac.pool())
    .await
    .expect("point");

    let routes: [(&str, String, Option<Value>); 5] = [
        (
            "GET",
            format!("/admin/negotiation/import?edition={slug}"),
            None,
        ),
        (
            "PUT",
            format!("/admin/negotiation/import?edition={slug}"),
            Some(reglage()),
        ),
        (
            "POST",
            format!("/admin/negotiation/import/read?edition={slug}"),
            None,
        ),
        (
            "GET",
            format!("/admin/negotiation/agenda-items?edition={slug}"),
            None,
        ),
        (
            "PUT",
            format!("/admin/negotiation/agenda-items/{point}"),
            Some(json!({ "theme": "gender" })),
        ),
    ];

    let app = back_office!(bac);
    let avant = ecritures(&bac).await;
    for (verbe, uri, corps) in &routes {
        let (statut, reponse) = frapper(
            &app,
            appel(verbe, uri, admin_cop, corps.clone()).to_request(),
        )
        .await;
        assert_eq!(
            (statut, reponse["code"].as_str()),
            (StatusCode::FORBIDDEN, Some("FORBIDDEN")),
            "{verbe} {uri} : l'administrateur de la COP31 ne passe pas"
        );
    }
    assert_eq!(ecritures(&bac).await, avant, "aucun refus n'a écrit");

    // Témoin : l'administratrice de la plateforme passe chaque garde.
    let attendus = [
        StatusCode::OK,
        StatusCode::OK,
        StatusCode::ACCEPTED,
        StatusCode::OK,
        StatusCode::OK,
    ];
    for ((verbe, uri, corps), attendu) in routes.iter().zip(attendus) {
        let (statut, _) = frapper(&app, appel(verbe, uri, ifdd, corps.clone()).to_request()).await;
        assert_eq!(statut, attendu, "{verbe} {uri} pour l'IFDD");
    }

    // Et le refus d'un réglage incomplet sort bien en 400, champ nommé.
    let mut incomplet = reglage();
    incomplet["reader"] = json!("live");
    let (statut, reponse) = frapper(
        &app,
        appel(
            "PUT",
            &format!("/admin/negotiation/import?edition={slug}"),
            ifdd,
            Some(incomplet),
        )
        .to_request(),
    )
    .await;
    assert_eq!(
        (statut, reponse["code"].as_str(), reponse["field"].as_str()),
        (
            StatusCode::BAD_REQUEST,
            Some("NEGOTIATION_IMPORT_CONFIG_INVALID"),
            Some("live_url")
        )
    );
}
