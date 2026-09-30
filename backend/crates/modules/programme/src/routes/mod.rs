//! Gestionnaires HTTP et formes de requête. Les formes de réponse vivent dans
//! `domain/` : elles sont la vue du modèle, pas celle du transport.

pub mod admin_desk;
pub mod admin_list;
pub mod admin_ops;
pub mod detail;
pub mod openapi;
pub mod people;
pub mod planner;
pub mod public_schedule;
pub mod questions;
pub mod registrations;
pub mod sessions;
pub mod submission;
pub mod workspace;

use actix_web::http::header::{ContentType, CACHE_CONTROL, ETAG, IF_NONE_MATCH};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use serde::Serialize;
use uuid::Uuid;

/// `200` portant l'empreinte du corps, ou `304` quand `If-None-Match` la
/// désigne : Guide Négo relit ces listes à chaque retour de réseau.
pub fn json_revalide<T: Serialize>(
    requete: &HttpRequest,
    corps: &T,
    cache: &'static str,
) -> Result<HttpResponse> {
    let texte = serde_json::to_string(corps).map_err(ApiError::internal)?;
    let empreinte = kernel::empreinte::de(&texte);
    let inchange = requete
        .headers()
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|presentee| kernel::empreinte::correspond(presentee, &empreinte));

    if inchange {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header((CACHE_CONTROL, cache))
            .finish());
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, cache))
        .content_type(ContentType::json())
        .body(texte))
}

/// La langue négociée par l'intergiciel, qui résout les textes du modèle.
/// Repli sur le français, comme `platform.t()`.
pub fn locale_de(requete: &HttpRequest) -> String {
    requete
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_else(|| "fr".to_owned())
}

/// Le contexte d'écriture, acteur posé. C'est lui qui alimente l'audit du
/// dossier — la seule table auditée du module (principe VII).
pub fn contexte_de(requete: &HttpRequest, acteur: Uuid) -> RequestContext {
    requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| {
            RequestContext::new(RequestContext::generated_request_id(), locale_de(requete))
        })
        .with_actor(acteur)
}

/// La personne connectée, **quand il y en a une** : pour les routes qui
/// répondent aussi sans session.
pub fn session_facultative(requete: &HttpRequest) -> Option<Uuid> {
    requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id)
}
