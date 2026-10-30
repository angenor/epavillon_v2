//! `GET /realtime/pulse?topics=…` : « quelque chose a changé ? », en quelques
//! octets. Le public y vient toutes les 30 s, une personne connectée toutes
//! les 15 s quand sa ligne ouverte est tombée.
//!
//! Servi de mémoire, sans toucher la base pour un sujet public ; un sujet privé
//! est autorisé par le même `TopicAuthorizer` que le flux, décision gardée
//! 60 s par session. Réponse `no-cache` avec ETag : 304 dans le cas courant.

use actix_web::http::header;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::realtime::{TopicRegistry, Viewer};
use serde::Serialize;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use super::hub::{Compteurs, Hub};
use super::stream::TopicsQuery;

#[derive(Debug, Serialize)]
pub struct RealtimePulse {
    /// Change au redémarrage de l'API ou à la reprise de son écoute : tout relire.
    pub boot: String,
    /// Version de chaque sujet accordé ; seule sa variation compte.
    pub versions: BTreeMap<String, u64>,
    pub denied: Vec<String>,
}

#[utoipa::path(
    get,
    description = "`RealtimePulse` — version courante des sujets demandés (`topics`, séparés par des virgules, 32 au plus). Sujets publics (`edition:<id>:public…`) sans session ; un sujet refusé est listé dans `denied`, et sans session c'est un 401 (le site rafraîchit puis réessaie). `Cache-Control: no-cache` et ETag : `If-None-Match` rend 304 tant que rien n'a changé.",
    path = "/realtime/pulse",
    tag = "Temps réel",
    operation_id = "realtime_pulse",
    params(("topics" = String, Query, description = "Sujets, séparés par des virgules")),
    responses(
        (status = 200, description = "RealtimePulse", body = Object),
        (status = 304, description = "Inchangé depuis l'ETag présenté"),
        (status = 401, description = "Sujet privé demandé sans session", body = crate::openapi::ApiErrorBody),
        (status = 422, description = "Sujet mal formé ou trop de sujets", body = crate::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn pouls(
    requete: HttpRequest,
    query: web::Query<TopicsQuery>,
    hub: web::Data<Hub>,
    registre: web::Data<TopicRegistry>,
    db: web::Data<Db>,
) -> Result<HttpResponse> {
    let viewer = requete
        .extensions()
        .get::<RequestContext>()
        .map(|c| Viewer {
            person_id: c.actor_id,
            session_id: c.session_id,
        })
        .unwrap_or(Viewer::ANONYMOUS);

    let sujets = super::lire_sujets(&query.topics)?;
    let (accordes, denied) =
        super::trier(&registre, db.pool(), &viewer, sujets, Some(&hub)).await?;
    Compteurs::plus(&hub.compteurs.refus, denied.len() as u64);
    if viewer.person_id.is_none() && !denied.is_empty() {
        return Err(ApiError::unauthenticated());
    }

    let corps = RealtimePulse {
        boot: hub.boot(),
        versions: hub.versions(&accordes),
        denied,
    };
    let json = serde_json::to_vec(&corps).map_err(ApiError::internal)?;
    let mut empreinte = DefaultHasher::new();
    json.hash(&mut empreinte);
    let etag = format!("\"{:016x}\"", empreinte.finish());

    Compteurs::plus(&hub.compteurs.pouls_servis, 1);
    let presentee = requete
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    if presentee.is_some_and(|p| p.split(',').any(|e| meme_etiquette(e, &etag))) {
        Compteurs::plus(&hub.compteurs.pouls_inchanges, 1);
        return Ok(HttpResponse::NotModified()
            .insert_header((header::ETAG, etag))
            .insert_header((header::CACHE_CONTROL, "no-cache"))
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((header::ETAG, etag))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .content_type("application/json")
        .body(json))
}

/// Apache, en compressant, suffixe l'ETag (`"…-br"`, `"…-gzip"`) et le
/// navigateur le renvoie tel quel : sans cette tolérance, jamais de 304.
fn meme_etiquette(presentee: &str, la_notre: &str) -> bool {
    let nette = |e: &str| {
        let e = e.trim().trim_start_matches("W/").trim_matches('"');
        e.strip_suffix("-br")
            .or_else(|| e.strip_suffix("-gzip"))
            .unwrap_or(e)
            .to_owned()
    };
    nette(presentee) == nette(la_notre)
}

#[cfg(test)]
mod tests {
    use super::meme_etiquette;

    #[test]
    fn etiquette_suffixee_par_apache() {
        assert!(meme_etiquette("\"00ab\"", "\"00ab\""));
        assert!(meme_etiquette("W/\"00ab-br\"", "\"00ab\""));
        assert!(meme_etiquette(" \"00ab-gzip\"", "\"00ab\""));
        assert!(!meme_etiquette("\"00ac\"", "\"00ab\""));
    }
}
