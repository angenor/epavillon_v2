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
pub mod registrations;
pub mod sessions;
pub mod submission;
pub mod workspace;

use actix_web::{HttpMessage, HttpRequest};
use kernel::context::RequestContext;
use uuid::Uuid;

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
