//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Les routes s'annotent auprès du gestionnaire qu'elles décrivent, les formes
//! de réponse sont désignées par leur nom TypeScript — leur source unique est
//! `frontend/app/types/` —, et le catalogue d'erreurs vient du noyau.

use utoipa::{OpenApi, ToSchema};

/// Forme du corps d'erreur, référencée par chaque route. Le schéma réel est
/// celui qu'engendre le noyau : l'API le repose après avoir fusionné les
/// documents des modules.
#[derive(ToSchema)]
#[schema(as = ApiError)]
#[allow(dead_code)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub request_id: Option<String>,
}

/// Les chemins s'ajoutent ici **au fil des histoires**, jamais d'avance : un
/// chemin décrit et non monté ferait rendre 404 à la documentation.
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::admin::lister,
        crate::routes::admin::gabarit_de_debordement,
        crate::routes::admin::relire,
        crate::routes::admin::creer,
        crate::routes::admin::corriger,
        crate::routes::admin::publier,
        crate::routes::admin::depublier,
        crate::routes::public::actifs,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Back-office — messages d'incident", description = "Ce qui se joue en ce moment, et ce qui en est déjà dit. Rédiger, publier, corriger, retirer — la trace reste. Filtré par le périmètre d'administration ; les quatre écritures exigent `live.incident.publish` **sur la portée visée**, et rendent leurs dix issues en 200."),
        (name = "Direct", description = "Ce que le public voit quand quelque chose ne se passe pas comme prévu : les messages actifs d'une édition, le plus grave en tête. Aucune garde — un bandeau d'incident est public par nature."),
    )
)]
pub struct LiveApi;
