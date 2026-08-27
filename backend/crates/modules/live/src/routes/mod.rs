pub mod admin;
pub mod openapi;
pub mod public;

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

/// Le contexte d'écriture, acteur posé.
///
/// **Ici, c'est une condition de JUSTESSE et non une trace** (principe VII) :
/// `live.publish_incident()` lit `platform.current_actor_id()` pour poser
/// `published_by`. Une écriture qui contournerait `Db::write()` produirait un
/// `published_by` NUL, sans erreur — et le back-office afficherait « publié
/// par — ». Une route gardée par `Perimeter` porte l'acteur sans passer par
/// `Actor`, d'où l'identifiant plutôt que l'extracteur.
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
