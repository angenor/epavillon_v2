pub mod acces;
pub mod admin_admission;
pub mod admin_codes;
pub mod admin_documents;
pub mod admin_import;
pub mod admin_requests;
pub mod agenda;
pub mod documents;
pub mod groups;
pub mod openapi;
pub mod sessions;
pub mod themes;

use actix_web::http::header::{HeaderName, CACHE_CONTROL};
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

/// Une réponse de session qui porte une empreinte dit l'état d'**une**
/// personne : aucun cache partagé ne la garde, et le navigateur la revalide.
pub const PERSONNEL: (HeaderName, &str) = (CACHE_CONTROL, "private, no-cache");

/// `If-None-Match` désigne-t-il l'état courant ? Voir `kernel::empreinte`.
pub fn inchange(requete: &HttpRequest, empreinte: &str) -> bool {
    entete(requete, actix_web::http::header::IF_NONE_MATCH.as_str())
        .is_some_and(|presentee| kernel::empreinte::correspond(&presentee, empreinte))
}

pub fn entete(requete: &HttpRequest, nom: &str) -> Option<String> {
    requete
        .headers()
        .get(nom)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}
