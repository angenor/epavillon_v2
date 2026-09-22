pub mod acces;
pub mod admin_admission;
pub mod admin_codes;
pub mod admin_requests;
pub mod openapi;
pub mod themes;

use actix_web::{HttpMessage, HttpRequest};
use kernel::context::RequestContext;
use uuid::Uuid;

/// La langue négociée par l'intergiciel. Repli sur le français, comme
/// `platform.t()`.
pub fn locale_de(requete: &HttpRequest) -> String {
    requete
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_else(|| "fr".to_owned())
}

/// Le contexte d'écriture, acteur posé — **et la session gardée avec lui**.
///
/// Elle sert à `invitation_code_uses.session_id` : c'est ce qui dira plus tard
/// d'où une entrée est partie. Une route gardée par une permission porte déjà
/// l'acteur, d'où l'identifiant en argument plutôt qu'un second extracteur.
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

pub fn entete(requete: &HttpRequest, nom: &str) -> Option<String> {
    requete
        .headers()
        .get(nom)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}
