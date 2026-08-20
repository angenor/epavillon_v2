//! Intergiciel de session : résout le jeton d'accès et remplit l'acteur du
//! contexte.
//!
//! La session est **relue en base à chaque requête**. C'est le prix de la
//! révocation immédiate : un jeton signé porte quinze minutes, mais une
//! suspension, une déconnexion ou une rotation cessent de valoir tout de suite
//! (FR-033). Rien ne serait gagné à s'en passer — toute route autorisée
//! interroge de toute façon `identity.has_permission()`.
//!
//! Une session absente ou périmée n'est pas un refus : elle laisse simplement le
//! contexte sans acteur, et c'est l'extracteur de la route qui décide — `GET
//! /auth/me` n'en veut aucun. **Une base injoignable, elle, est une panne** :
//! la confondre avec une absence de session annoncerait « déconnecté » à
//! quelqu'un qui ne l'est pas, et le renverrait se connecter en vain. Le refus
//! ne tombe alors que sur les requêtes qui présentaient un cookie de session ;
//! un visiteur anonyme n'a rien à quoi la panne s'applique.

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error, HttpMessage, ResponseError};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use identity::AccessTokenCodec;
use kernel::context::RequestContext;
use kernel::db::Db;
use std::rc::Rc;

pub struct SessionResolver;

impl<S, B> Transform<S, ServiceRequest> for SessionResolver
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = SessionResolverService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SessionResolverService {
            service: Rc::new(service),
        }))
    }
}

pub struct SessionResolverService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SessionResolverService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let jeton = req
            .cookie(identity::COOKIE_ACCES)
            .map(|c| c.value().to_owned())
            .filter(|v| !v.is_empty());
        let pool = req.app_data::<web::Data<Db>>().map(|db| db.pool().clone());
        let codec = req
            .app_data::<web::Data<AccessTokenCodec>>()
            .map(|c| c.clone().into_inner());
        let service = self.service.clone();

        Box::pin(async move {
            if let (Some(jeton), Some(pool), Some(codec)) = (jeton, pool, codec) {
                match identity::resolve_actor(&pool, &codec, &jeton).await {
                    Ok(Some(acteur)) => {
                        let contexte = req.extensions().get::<RequestContext>().cloned();
                        if let Some(contexte) = contexte {
                            req.extensions_mut().insert(contexte.with_actor(acteur));
                        }
                    }
                    Ok(None) => {}
                    Err(erreur) => {
                        let reponse = erreur.error_response();
                        return Ok(req.into_response(reponse).map_into_right_body());
                    }
                }
            }
            service
                .call(req)
                .await
                .map(ServiceResponse::map_into_left_body)
        })
    }
}
