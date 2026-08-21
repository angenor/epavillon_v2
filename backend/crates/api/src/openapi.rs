//! Le document OpenAPI de l'API entière — **assemblé**, jamais écrit.
//!
//! Chaque module apporte le sien ; l'API y ajoute ses propres routes
//! d'exploitation, l'adresse de base et la façon dont la session voyage. Un
//! module démonté (`platform.modules`) ne monte pas ses routes : sa part de la
//! documentation ne se sert pas non plus, sans quoi elle décrirait des chemins
//! qui rendent 404.
//!
//! **La documentation est servie en JSON, pas en interface web.** Une interface
//! embarquée téléchargerait ses fichiers à la compilation — une dépendance
//! réseau dans le portail de vérification, pour un confort que n'importe quel
//! lecteur d'OpenAPI apporte déjà. Le document, lui, est le livrable.

use actix_web::{web, HttpResponse};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{Components, InfoBuilder, OpenApi as Document, ServerBuilder};
use utoipa::{OpenApi, ToSchema};

use crate::modules::ModuleRegistry;
use crate::PREFIXE;

/// La forme du corps d'erreur, pour les routes que l'API porte elle-même. Le
/// catalogue complet des codes est injecté par le document du module
/// `identity`, qui le tient du noyau.
#[derive(ToSchema)]
#[schema(as = ApiError)]
#[allow(dead_code)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub request_id: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    paths(crate::routes::health::ready, crate::routes::health::health),
    components(schemas(ApiErrorBody)),
    tags((name = "Exploitation", description = "Vivacité et santé. `/ready` ne divulgue rien ; `/health` porte des chiffres et se protège comme une donnée."))
)]
struct ApiRoutes;

/// Assemble le document d'après ce qui est **réellement monté**.
pub fn document(modules: &ModuleRegistry) -> Document {
    let mut doc = ApiRoutes::openapi();

    doc.info = InfoBuilder::new()
        .title("ePavillon v2 — API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some(
            "Plateforme numérique de l'IFDD.\n\n\
             **Un refus prévu par le contrat du site n'est pas une erreur HTTP** : il sort en 200 \
             avec son discriminant `status`. Les statuts d'erreur sont réservés à ce que le \
             contrat n'exprime pas — et portent alors un corps `ApiError`.\n\n\
             Les formes de réponse sont désignées par leur nom TypeScript : leur source unique \
             est `frontend/app/types/`.",
        ))
        .build();

    doc.servers = Some(vec![ServerBuilder::new()
        .url(PREFIXE)
        .description(Some("Préfixe de toutes les routes"))
        .build()]);

    if modules.is_mounted("identity") {
        doc.merge(identity::routes::openapi::IdentityApi::openapi());
    }
    if modules.is_mounted("org") {
        doc.merge(org::routes::openapi::OrgApi::openapi());
    }
    if modules.is_mounted("event") {
        doc.merge(event::routes::openapi::EventApi::openapi());
    }
    if modules.is_mounted("programme") {
        doc.merge(programme::routes::openapi::ProgrammeApi::openapi());
    }

    // Le catalogue complet des codes vient du noyau, par le module qui sait le
    // rendre. Il est reposé APRÈS la fusion : `merge` garde le schéma déjà
    // présent, et la déclaration plate ci-dessus l'emporterait.
    let composants = doc.components.get_or_insert_with(Components::new);
    composants.schemas.insert(
        "ApiError".to_owned(),
        identity::routes::openapi::schema_du_catalogue_derreurs(),
    );

    // La session voyage par cookie : le déclarer évite qu'un lecteur cherche un
    // en-tête d'autorisation qui n'existe pas.
    composants.add_security_scheme(
        "session",
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            identity::COOKIE_ACCES,
            "Jeton d'accès, posé par `/auth/login` et renouvelé par `/auth/refresh`. Le site l'envoie seul (`credentials: 'include'`).",
        ))),
    );

    doc
}

/// `GET /docs` — le document généré.
///
/// Il est composé **une fois** au montage de l'application : l'assembler à
/// chaque appel ferait payer la sérialisation du document entier à qui sonde la
/// route.
pub fn configurer(cfg: &mut web::ServiceConfig, document: &Document) {
    let rendu = document
        .to_json()
        .unwrap_or_else(|e| format!(r#"{{"error":"document illisible: {e}"}}"#));

    cfg.route(
        "/docs",
        web::get().to(move || {
            let rendu = rendu.clone();
            async move {
                HttpResponse::Ok()
                    .content_type("application/json; charset=utf-8")
                    .body(rendu)
            }
        }),
    );
}
