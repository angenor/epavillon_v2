//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Même parti que dans les modules livrés : les routes s'annotent auprès du
//! gestionnaire qu'elles décrivent, les formes de réponse sont désignées par
//! leur nom TypeScript — leur source unique est `frontend/app/types/` —, et le
//! catalogue d'erreurs vient du noyau.

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
        crate::routes::public::vitrine,
        crate::routes::admin::lister,
        crate::routes::admin::formulaire_vierge,
        crate::routes::admin::formulaire,
        crate::routes::admin::valeurs,
        crate::routes::admin::seances,
        crate::routes::admin::creer,
        crate::routes::admin::modifier,
        crate::routes::admin::changer_le_statut,
        crate::routes::admin::deplacer,
        crate::routes::admin::dupliquer,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Vitrine", description = "Ce que le public voit en arrivant : le bandeau d'ouverture de l'accueil, composé depuis le back-office."),
        (name = "Back-office — vitrine", description = "Composer le bandeau d'accueil : créer, modifier, publier, ordonner, dupliquer. Filtré par le périmètre d'administration ; un contenu de plateforme exige la portée globale."),
    )
)]
pub struct ContentApi;
