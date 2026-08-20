//! Gestionnaires HTTP et formes de requête. Les formes de réponse, elles,
//! vivent dans `domain/` : elles sont la vue du modèle, pas celle du transport.

pub mod admin_privacy;
pub mod admin_users;
pub mod auth;
pub mod cookies;
pub mod openapi;
pub mod people;

use actix_web::{HttpMessage, HttpRequest};
use kernel::context::RequestContext;

/// La langue négociée par l'intergiciel, qui résout les textes du modèle.
/// Repli sur le français, comme `platform.t()`.
pub fn locale_de(requete: &HttpRequest) -> String {
    requete
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_else(|| "fr".to_owned())
}
