//! Plateforme en maintenance : seules les personnes qui administrent écrivent.
//!
//! Le drapeau `platform.maintenance` ferme la plateforme le temps d'une reprise
//! de données. Une inscription ou un dépôt fait pendant le transfert se
//! heurterait ensuite aux lignes importées. Le site renvoie déjà les visiteurs
//! vers la page d'attente ; ce verrou tient aussi pour qui appelle l'API sans
//! passer par lui.
//!
//! Les lectures passent : le site lit les drapeaux et la session avant de savoir
//! qui regarde. La connexion et le mot de passe oublié aussi, sans quoi aucun
//! administrateur n'entrerait.
//!
//! « Administrer » a le sens du back-office : un périmètre non vide
//! (`identity.administered_events`), global ou limité à une édition.

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::Method;
use actix_web::{web, Error, HttpMessage, ResponseError};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::PgPool;
use std::rc::Rc;
use uuid::Uuid;

use crate::routes::platform::resoudre;
use crate::PREFIXE;

pub const DRAPEAU: &str = "platform.maintenance";

// Les notifications de Zoom ne sont pas une écriture de personne : les refuser
// pendant la maintenance perdrait les présences, Zoom ne les réémet pas.
const ECRITURES_OUVERTES: [&str; 6] = [
    "/auth/login",
    "/auth/logout",
    "/auth/refresh",
    "/auth/password-reset",
    "/auth/password-reset/confirm",
    "/webhooks/zoom",
];

pub struct MaintenanceGate;

impl<S, B> Transform<S, ServiceRequest> for MaintenanceGate
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = MaintenanceGateService<S>;
    type InitError = ();
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MaintenanceGateService {
            service: Rc::new(service),
        }))
    }
}

pub struct MaintenanceGateService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for MaintenanceGateService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let a_controler = est_ecriture(req.method()) && !ecriture_ouverte(req.path());
        let pool = req.app_data::<web::Data<Db>>().map(|db| db.pool().clone());
        let acteur = req
            .extensions()
            .get::<RequestContext>()
            .and_then(|ctx| ctx.actor_id);
        let service = self.service.clone();

        Box::pin(async move {
            if let (true, Some(pool)) = (a_controler, pool) {
                let refus = match refusee(&pool, acteur).await {
                    Ok(false) => None,
                    Ok(true) => Some(ApiError::new(ErrorCode::PlatformMaintenance)),
                    Err(erreur) => Some(erreur),
                };
                if let Some(refus) = refus {
                    return Ok(req
                        .into_response(refus.error_response())
                        .map_into_right_body());
                }
            }
            service
                .call(req)
                .await
                .map(ServiceResponse::map_into_left_body)
        })
    }
}

fn est_ecriture(methode: &Method) -> bool {
    !matches!(
        *methode,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn ecriture_ouverte(chemin: &str) -> bool {
    chemin
        .strip_prefix(PREFIXE)
        .map(|reste| reste.trim_end_matches('/'))
        .is_some_and(|reste| ECRITURES_OUVERTES.contains(&reste))
}

async fn refusee(pool: &PgPool, acteur: Option<Uuid>) -> Result<bool> {
    // Résolu sans personne : la maintenance vaut pour tous, seul un drapeau
    // déployé à 100 % la déclenche.
    let fermee = resoudre(pool, None)
        .await?
        .iter()
        .any(|drapeau| drapeau.key == DRAPEAU && drapeau.is_enabled);
    if !fermee {
        return Ok(false);
    }
    let Some(acteur) = acteur else {
        return Ok(true);
    };
    Ok(kernel::auth::administered_events(pool, acteur)
        .await?
        .is_empty())
}
