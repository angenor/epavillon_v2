//! Gestionnaires HTTP et formes de requête. Les formes de réponse vivent dans
//! `domain/` : elles sont la vue du modèle, pas celle du transport.

pub mod admin_call;
pub mod admin_committee;
pub mod admin_events;
pub mod admin_tabs;
pub mod openapi;
pub mod planner;
pub mod public;

use actix_web::{HttpMessage, HttpRequest};
use kernel::auth::Actor;
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

/// Le contexte d'écriture, acteur posé. C'est lui qui alimente l'audit de
/// l'édition et de l'appel — les deux seules tables auditées du module
/// (principe VII).
pub fn contexte(requete: &HttpRequest, acteur: Actor) -> RequestContext {
    contexte_de(requete, acteur.0)
}

/// Le contexte d'écriture, acteur donné par son identifiant — ce que fait une
/// route gardée par `Perimeter`, qui porte l'acteur sans passer par `Actor`.
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
