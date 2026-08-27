pub mod admin;
pub mod openapi;

use actix_web::{HttpMessage, HttpRequest};
use kernel::context::RequestContext;

/// La langue négociée par l'intergiciel. Repli sur le français, comme
/// `platform.t()`.
pub fn locale_de(requete: &HttpRequest) -> String {
    requete
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_else(|| "fr".to_owned())
}
