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

/// Les chemins s'ajoutent ici **au fil des récits**, jamais d'avance : un chemin
/// décrit et non monté ferait rendre 404 à la documentation.
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::acces::mon_acces,
        crate::routes::acces::saisir_un_code,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Guide Négo — accès", description = "Entrer dans les modules réservés : le code d'invitation, l'état de son propre accès, la demande à trancher."),
        (name = "Back-office — admission", description = "Les codes d'invitation, leurs usages, les demandes d'accès et le mode d'admission. Gardé par le périmètre et par `negotiation.space.manage` sur la portée demandée."),
    )
)]
pub struct NegotiationApi;
