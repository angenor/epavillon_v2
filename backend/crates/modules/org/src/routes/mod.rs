//! Gestionnaires HTTP et formes de requête. Les formes de réponse vivent dans
//! `domain/` : elles sont la vue du modèle, pas celle du transport.

pub mod admin;
pub mod memberships;
pub mod openapi;
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

/// Le contexte d'écriture, acteur posé. C'est lui qui alimente l'audit et
/// l'historique de la fiche (principe VII).
pub fn contexte(requete: &HttpRequest, acteur: Actor) -> RequestContext {
    contexte_sans_acteur(requete).with_actor(acteur.0)
}

/// Le contexte d'écriture, acteur donné par son identifiant — ce que fait une
/// route gardée par `Perimeter`, qui porte l'acteur sans passer par `Actor`.
pub fn contexte_de(requete: &HttpRequest, acteur: Uuid) -> RequestContext {
    contexte_sans_acteur(requete).with_actor(acteur)
}

/// Le contexte d'une route qui n'exige pas de session — l'acceptation d'une
/// invitation. L'acteur y est posé **en cours de transaction**, quand le jeton
/// consommé a dit qui il désigne.
pub fn contexte_sans_acteur(requete: &HttpRequest) -> RequestContext {
    requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| {
            RequestContext::new(RequestContext::generated_request_id(), locale_de(requete))
        })
}

/// La personne connectée, **s'il y en a une**. Une route qui n'exige pas de
/// session ne peut pas se servir de l'extracteur : il rendrait 401.
pub fn acteur_optionnel(requete: &HttpRequest) -> Option<Uuid> {
    requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id)
}
