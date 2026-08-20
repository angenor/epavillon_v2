//! Intergiciel de contexte de requête.
//!
//! Lit `X-Request-Id` s'il est présent, l'engendre sinon, négocie la locale,
//! pose le contexte dans la requête et dans la portée de tâche — c'est ce qui
//! permet à une réponse d'erreur de porter le même identifiant —, et le renvoie
//! dans l'en-tête de réponse.

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use kernel::context::{RequestContext, REQUEST_ID_HEADER};
use kernel::i18n::Locales;
use std::rc::Rc;
use tracing::Instrument;

/// Borne large : un identifiant venu du client ne doit ni gonfler les journaux
/// ni transporter autre chose que des caractères d'en-tête.
const LONGUEUR_MAX: usize = 128;

pub struct RequestContextMiddleware {
    locales: Locales,
}

impl RequestContextMiddleware {
    pub fn new(locales: Locales) -> Self {
        Self { locales }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestContextMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestContextService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestContextService {
            service: Rc::new(service),
            locales: self.locales.clone(),
        }))
    }
}

pub struct RequestContextService<S> {
    service: Rc<S>,
    locales: Locales,
}

impl<S, B> Service<ServiceRequest> for RequestContextService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim())
            .filter(|v| {
                !v.is_empty()
                    && v.len() <= LONGUEUR_MAX
                    && v.chars().all(|c| c.is_ascii_graphic() || c == ' ')
            })
            .map(str::to_owned)
            .unwrap_or_else(RequestContext::generated_request_id);

        let locale = self.locales.negotiate(
            req.headers()
                .get(actix_web::http::header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok()),
        );

        req.extensions_mut()
            .insert(RequestContext::new(request_id.clone(), locale));

        let service = self.service.clone();
        let entete = request_id.clone();

        // Sans span, l'exportateur OTLP n'a rien à exporter : la couche
        // OpenTelemetry ignore tout événement hors span, et Jaeger reste vide.
        let span = tracing::info_span!(
            "http",
            methode = %req.method(),
            chemin = req.path(),
            request_id = %request_id
        );

        Box::pin(kernel::context::scope(
            request_id,
            async move {
                let mut res = service.call(req).await?;
                if let Ok(valeur) = HeaderValue::from_str(&entete) {
                    res.headers_mut()
                        .insert(HeaderName::from_static("x-request-id"), valeur);
                }
                Ok(res)
            }
            .instrument(span),
        ))
    }
}
