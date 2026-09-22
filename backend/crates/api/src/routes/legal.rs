//! Les textes qui engagent — **une source, deux lecteurs** : le site et Guide Négo
//! appellent la même route, et opposent donc le même texte et la même version.
//!
//! **Aucune session n'est exigée** : on lit une politique de confidentialité
//! avant d'avoir un compte. La route n'appartient à aucun module, comme le
//! référentiel.
//!
//! **Les corps se sérialisent une fois, au montage** : un texte embarqué dans le
//! binaire ne change pas entre deux démarrages. Un texte que l'IFDD n'a pas encore
//! fourni part avec `status: "pending"` et sans corps — l'écran le dit.

use std::collections::HashMap;

use actix_web::http::header::{CACHE_CONTROL, CONTENT_LANGUAGE, ETAG, VARY};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::legal::{self, CLES, LANGUES};

pub(crate) struct Servi {
    corps: String,
    empreinte: String,
    langue: &'static str,
}

/// `(clé, langue demandée)` → ce qui part. La langue demandée qui n'a pas de
/// fichier retombe sur le français, et la réponse dit la langue servie.
pub(crate) struct TextesServis(HashMap<(&'static str, &'static str), Servi>);

pub fn configurer(cfg: &mut web::ServiceConfig) {
    let mut servis = HashMap::new();
    for cle in CLES {
        for langue in LANGUES {
            let texte = legal::texte(cle, langue).expect("texte embarqué");
            let corps = serde_json::to_string(texte).expect("texte sérialisable");
            let empreinte = kernel::empreinte::de(&corps);
            servis.insert(
                (cle, langue),
                Servi {
                    corps,
                    empreinte,
                    langue: texte.locale,
                },
            );
        }
    }
    cfg.app_data(web::Data::new(TextesServis(servis)))
        .route("/legal/{cle}", web::get().to(texte));
}

#[utoipa::path(
    get,
    description = "`LegalText` — la politique de confidentialité (`privacy`) ou les conditions d'utilisation (`terms`), **la même source pour le site et pour Guide Négo**. Sans session.\n\nLa langue suit `Accept-Language`, **avec repli sur le français** ; `locale` et `Content-Language` disent la langue servie. `version` est celle qu'un consentement enregistre.\n\nTant que l'IFDD n'a pas fourni le texte : `status: \"pending\"`, `body` et `effective_date` nuls, et la version reste `2026-01`. Publié : `status: \"published\"`, le corps en Markdown dans une grammaire close, et **la date d'entrée en vigueur pour version**.\n\n`ETag` sur le corps rendu, `304` sur `If-None-Match`.",
    path = "/legal/{cle}",
    tag = "Plateforme",
    operation_id = "legal_texte",
    params(("cle" = String, Path, description = "privacy ou terms")),
    responses(
        (status = 200, description = "LegalText", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 404, description = "Clé inconnue", body = crate::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn texte(
    servis: web::Data<TextesServis>,
    requete: HttpRequest,
    cle: web::Path<String>,
) -> Result<HttpResponse> {
    let demandee = requete
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_default();
    let langue = LANGUES.into_iter().find(|l| *l == demandee).unwrap_or("fr");
    let cle = CLES
        .into_iter()
        .find(|c| *c == cle.as_str())
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
    let servi = &servis.0[&(cle, langue)];

    let inchange = requete
        .headers()
        .get(actix_web::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|presentee| kernel::empreinte::correspond(presentee, &servi.empreinte));

    let mut reponse = if inchange {
        HttpResponse::NotModified()
    } else {
        HttpResponse::Ok()
    };
    reponse
        .insert_header((ETAG, servi.empreinte.clone()))
        .insert_header((CONTENT_LANGUAGE, servi.langue))
        .insert_header((VARY, "Accept-Language"))
        // Public, mais revalidé : un nouveau binaire peut porter une nouvelle version.
        .insert_header((CACHE_CONTROL, "public, no-cache"));

    if inchange {
        return Ok(reponse.finish());
    }
    Ok(reponse
        .content_type("application/json")
        .body(servi.corps.clone()))
}
