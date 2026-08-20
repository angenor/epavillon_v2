//! Vérification de l'en-tête d'origine sur toute écriture.
//!
//! La protection contre la requête forgée tient à `SameSite` **plus** ce
//! contrôle. Pas de jeton anti-CSRF en en-tête : aucun écran n'en envoie,
//! l'exiger casserait le front sans arbitrage (research.md § R2).
//!
//! Le contrôle porte sur l'**origine annoncée** : l'en-tête `Origin`, ou à
//! défaut le schéma et l'autorité du `Referer`. Une écriture sans aucun des
//! deux passe — les navigateurs posent un `Origin` sur toute écriture, donc
//! l'absence des deux désigne un client qui n'est pas un navigateur, et qui
//! n'est pas le vecteur qu'on ferme ici. Une origine annoncée et inconnue est
//! refusée, `null` compris : une valeur littérale `null` vient d'une iframe
//! cloisonnée ou d'une redirection inter-schémas, ce n'est pas une absence.

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{ORIGIN, REFERER};
use actix_web::http::Method;
use actix_web::{Error, ResponseError};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use kernel::error::{ApiError, ErrorCode};
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone)]
pub struct OriginCheck {
    autorisees: Rc<HashSet<String>>,
}

impl OriginCheck {
    pub fn new(origines: impl IntoIterator<Item = String>) -> Self {
        Self {
            autorisees: Rc::new(origines.into_iter().map(|o| normaliser(&o)).collect()),
        }
    }
}

/// Partagée avec l'intergiciel CORS : les deux doivent comparer les origines
/// de la même façon, sans quoi l'un autorise ce que l'autre refuse.
pub(crate) fn normaliser(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

/// Ne garde que le schéma et l'autorité d'un `Referer` : `https://site/x/y`
/// devient `https://site`.
fn origine_du_referent(referer: &str) -> Option<String> {
    let (schema, reste) = referer.split_once("://")?;
    let autorite = reste.split('/').next()?;
    Some(normaliser(&format!("{schema}://{autorite}")))
}

impl<S, B> Transform<S, ServiceRequest> for OriginCheck
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = OriginCheckService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(OriginCheckService {
            service: Rc::new(service),
            autorisees: self.autorisees.clone(),
        }))
    }
}

pub struct OriginCheckService<S> {
    service: Rc<S>,
    autorisees: Rc<HashSet<String>>,
}

impl<S, B> Service<ServiceRequest> for OriginCheckService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let ecriture = !matches!(
            *req.method(),
            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
        );

        let annoncee = req
            .headers()
            .get(ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(normaliser)
            .or_else(|| {
                req.headers()
                    .get(REFERER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(origine_du_referent)
            });

        let refusee = ecriture
            && annoncee
                .as_deref()
                .is_some_and(|o| !self.autorisees.contains(o));

        if refusee {
            let reponse = ApiError::new(ErrorCode::IdentityOriginRejected).error_response();
            let (requete, _) = req.into_parts();
            return Box::pin(ready(Ok(
                ServiceResponse::new(requete, reponse).map_into_right_body()
            )));
        }

        let service = self.service.clone();
        Box::pin(async move { service.call(req).await.map(|r| r.map_into_left_body()) })
    }
}
