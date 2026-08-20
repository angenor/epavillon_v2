//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Même parti qu'en B1 : les routes s'annotent auprès du gestionnaire qu'elles
//! décrivent, les formes de réponse sont désignées par leur nom TypeScript —
//! leur source unique est `frontend/app/types/` —, et le catalogue d'erreurs
//! vient du noyau, code par code.

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

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::public::similaires,
        crate::routes::public::par_domaine_dadresse,
        crate::routes::memberships::rejoindre,
        crate::routes::memberships::mes_adhesions,
        crate::routes::memberships::inviter,
        crate::routes::memberships::decider,
        crate::routes::memberships::accepter_invitation,
        crate::routes::memberships::revoquer,
        crate::routes::public::lister,
        crate::routes::public::creer,
        crate::routes::public::fiche,
        crate::routes::admin::similaires_non_filtrees,
        crate::routes::admin::liste,
        crate::routes::admin::fiche,
        crate::routes::admin::verification,
        crate::routes::admin::domaine,
        crate::routes::admin::denomination,
        crate::routes::admin::file_des_doublons,
        crate::routes::admin::decision_de_doublon,
        crate::routes::admin::apercu_de_fusion,
        crate::routes::admin::fusionner,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Organisations", description = "Recherche, rattachement et création. Session requise, et rien de plus : la permission de consultation est détenue par le rôle d'utilisateur ordinaire, que rien n'attribue encore."),
        (name = "Back-office — organisations", description = "Permission de consultation ET périmètre d'administration non vide. Une organisation n'appartient à aucune édition : le périmètre se lit par l'activité déposée ou tenue."),
    )
)]
pub struct OrgApi;

#[cfg(test)]
mod tests {
    use super::*;

    /// **Les vingt et une routes du module sont décrites.**
    ///
    /// Le compte est écrit pour qu'une route ajoutée sans son annotation fasse
    /// échouer ce test plutôt que de passer inaperçue. Vingt chemins seulement :
    /// `/organizations` en porte deux — la liste et la création.
    #[test]
    fn les_routes_du_module_sont_toutes_decrites() {
        let document = OrgApi::openapi();
        assert_eq!(document.paths.paths.len(), 20, "chemins distincts");

        let rendu = document.to_json().expect("sérialisation du document");
        let chemins: serde_json::Value = serde_json::from_str(&rendu).expect("document JSON");
        let operations: usize = chemins["paths"]
            .as_object()
            .expect("les chemins")
            .values()
            .map(|c| c.as_object().expect("un chemin").len())
            .sum();
        assert_eq!(operations, 21, "opérations");
    }

    /// **Les onze codes du module sont au catalogue du noyau.** Ils sont
    /// engendrés dans la documentation depuis lui : un code ajouté apparaît au
    /// prochain démarrage, un code oublié n'existe pas.
    #[test]
    fn les_onze_codes_du_module_sont_au_catalogue() {
        use kernel::error::ErrorCode;

        let du_module: Vec<&str> = ErrorCode::ALL
            .iter()
            .map(|c| c.as_str())
            .filter(|c| c.starts_with("ORG_"))
            .collect();

        assert_eq!(du_module.len(), 11, "onze codes : {du_module:?}");
    }
}
