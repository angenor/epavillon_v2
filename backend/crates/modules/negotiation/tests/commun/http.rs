//! Le back-office du module monté seul dans une application d'essai, sans
//! `api` : un intergiciel pose l'acteur à la place de la session.

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::{Method, StatusCode};
use actix_web::test::{call_service, read_body, TestRequest};
use serde_json::Value;
use uuid::Uuid;

use super::Bac;

/// L'en-tête qui tient lieu de session : l'application d'essai n'a pas
/// l'intergiciel de `api`, seulement ce qu'il pose — le contexte et son acteur.
pub const ACTEUR: &str = "x-essai-acteur";

/// Le back-office du module, monté seul dans une application d'essai.
#[macro_export]
macro_rules! back_office {
    ($bac:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($bac.db()))
                .app_data(actix_web::web::Data::new($bac.state.clone()))
                .wrap_fn(|req, srv| {
                    let acteur = req
                        .headers()
                        .get($crate::commun::http::ACTEUR)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| uuid::Uuid::parse_str(v).ok());
                    let ctx = kernel::context::RequestContext::new(
                        kernel::context::RequestContext::generated_request_id(),
                        "fr",
                    );
                    actix_web::HttpMessage::extensions_mut(&req).insert(match acteur {
                        Some(a) => ctx.with_actor(a),
                        None => ctx,
                    });
                    actix_web::dev::Service::call(srv, req)
                })
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

pub async fn une_edition(bac: &Bac, slug: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'), $1::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

pub fn appel(verbe: &str, uri: &str, acteur: Option<Uuid>, corps: Option<Value>) -> TestRequest {
    let methode = Method::from_bytes(verbe.to_uppercase().as_bytes()).expect("verbe HTTP");
    let mut r = TestRequest::default().method(methode).uri(uri);
    if let Some(a) = acteur {
        r = r.insert_header((ACTEUR, a.to_string()));
    }
    if let Some(c) = corps {
        r = r.set_json(c);
    }
    r
}

pub async fn frapper<S, R, B>(app: &S, requete: R) -> (StatusCode, Value)
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
