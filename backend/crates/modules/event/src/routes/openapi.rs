//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Même parti qu'en B1 et B2 : les routes s'annotent auprès du gestionnaire
//! qu'elles décrivent, les formes de réponse sont désignées par leur nom
//! TypeScript — leur source unique est `frontend/app/types/` —, et le catalogue
//! d'erreurs vient du noyau, code par code.

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
        crate::routes::admin_events::options_de_formulaire,
        crate::routes::admin_events::lister,
        crate::routes::admin_events::detail,
        crate::routes::admin_events::creer,
        crate::routes::admin_events::modifier,
        crate::routes::admin_tabs::creer_lieu,
        crate::routes::admin_tabs::modifier_lieu,
        crate::routes::admin_tabs::supprimer_lieu,
        crate::routes::admin_tabs::creer_salle,
        crate::routes::admin_tabs::modifier_salle,
        crate::routes::admin_tabs::supprimer_salle,
        crate::routes::admin_tabs::creer_canal,
        crate::routes::admin_tabs::modifier_canal,
        crate::routes::admin_tabs::supprimer_canal,
        crate::routes::admin_tabs::plan_des_journees,
        crate::routes::admin_tabs::generer_les_journees,
        crate::routes::admin_tabs::habiller_une_journee,
        crate::routes::admin_tabs::creer_fil,
        crate::routes::admin_tabs::modifier_fil,
        crate::routes::admin_tabs::supprimer_fil,
        crate::routes::admin_call::grille_par_defaut,
        crate::routes::admin_call::creer,
        crate::routes::admin_call::modifier,
        crate::routes::admin_committee::enregistrer,
        crate::routes::planner::controle,
        crate::routes::planner::publier,
        crate::routes::public::selecteur_des_editions,
        crate::routes::public::editions_publiques,
        crate::routes::public::edition_publique,
        crate::routes::public::series,
        crate::routes::public::journees,
        crate::routes::public::fils,
        crate::routes::public::lieux,
        crate::routes::public::salles,
        crate::routes::public::canaux,
        crate::routes::public::appel,
        crate::routes::public::images,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Événements", description = "Lectures publiques : éditions annoncées, séries, page d'une édition. Aucune session requise."),
        (name = "Back-office — événements", description = "Permission de gestion des événements ET périmètre d'administration non vide. Un objet enfant remonte à son édition AVANT que le périmètre soit vérifié."),
        (name = "Back-office — appel à propositions", description = "Permission de gestion des appels, distincte de celle des événements : un compte peut tenir le décor d'une édition sans ouvrir sa campagne."),
        (name = "Planificateur", description = "Contrôle préalable et publication de la programmation. Gardés par la permission de PLANIFIER, celle que le modèle attribue au rôle chargé de publier le programme."),
    )
)]
pub struct EventApi;

#[cfg(test)]
mod tests {
    /// **Les trois codes du module sont au catalogue du noyau.** Ils sont
    /// engendrés dans la documentation depuis lui : un code ajouté apparaît au
    /// prochain démarrage, un code oublié n'existe pas.
    #[test]
    fn les_trois_codes_du_module_sont_au_catalogue() {
        use kernel::error::ErrorCode;

        let du_module: Vec<&str> = ErrorCode::ALL
            .iter()
            .map(|c| c.as_str())
            .filter(|c| c.starts_with("EVENT_"))
            .collect();

        assert_eq!(
            du_module,
            [
                "EVENT_GLOBAL_SCOPE_REQUIRED",
                "EVENT_CRITERION_HAS_SCORES",
                "EVENT_UNKNOWN_REFERENCE"
            ],
            "trois codes, et ceux-là"
        );
    }

    /// Chacun porte son statut : un refus de portée n'est pas une validation,
    /// et le front branche sur le couple code–statut.
    #[test]
    fn chaque_code_porte_son_statut() {
        use actix_web::http::StatusCode;
        use kernel::error::ErrorCode;

        assert_eq!(
            ErrorCode::EventGlobalScopeRequired.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::EventCriterionHasScores.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ErrorCode::EventUnknownReference.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
