//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Même parti qu'en B1 à B5 : les routes s'annotent auprès du gestionnaire
//! qu'elles décrivent, et les formes de réponse sont désignées par leur nom
//! TypeScript — leur source unique est `frontend/app/types/`.

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
        crate::routes::uploads::annoncer,
        crate::routes::uploads::deposer,
        crate::routes::assets::objet,
        crate::routes::assets::avancement,
        crate::routes::assets::supprimer,
        crate::routes::roles::roles,
        crate::routes::attachments::lister,
        crate::routes::attachments::poser,
        crate::routes::attachments::remplacer,
        crate::routes::attachments::detacher,
        crate::routes::admin::orphelins,
        crate::routes::admin::quotas,
        crate::routes::admin::relever_le_plafond,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Média — dépôt", description = "L'annonce préalable, le dépôt en flux, la description d'un objet et son avancement. **Aucune permission `media.*` n'existe** : le droit de poser un fichier est le droit d'écrire sur ce qu'il illustre. Un fichier déjà connu rend l'objet existant en 200 — c'est le succès de la déduplication, pas un refus."),
        (name = "Média — rattachements", description = "Poser, remplacer, retirer. La table blanche `media.attachable_roles` déclare la forme attendue ; elle ne déclare pas qui a le droit, et une combinaison sans garde est refusée. Détacher **ne supprime pas l'objet** : le champ de réponse le dit, parce que c'est la question qu'on se pose en lisant."),
        (name = "Back-office — médias", description = "Orphelins et quotas. Gardé par `org.organization.manage` sur la portée globale : un compte sans aucun périmètre reçoit un refus explicite, jamais une liste vide."),
    )
)]
pub struct MediaApi;
