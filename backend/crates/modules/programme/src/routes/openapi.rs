//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.
//!
//! Même parti qu'en B1, B2 et B3 : les routes s'annotent auprès du gestionnaire
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
        crate::routes::submission::contexte_du_formulaire,
        crate::routes::submission::mon_brouillon,
        crate::routes::submission::creer,
        crate::routes::submission::modifier,
        crate::routes::submission::deposer,
        crate::routes::submission::rouvrir,
        crate::routes::submission::renvoyer,
        crate::routes::people::chercher,
        crate::routes::detail::regles,
        crate::routes::detail::journal,
        crate::routes::detail::transitions_offertes,
        crate::routes::detail::de_lorganisation,
        crate::routes::detail::dossier,
        crate::routes::detail::organisations,
        crate::routes::detail::intervenants,
        crate::routes::detail::thematiques,
        crate::routes::detail::historique,
        crate::routes::detail::fil,
        crate::routes::detail::pieces,
        crate::routes::detail::rattacher,
        crate::routes::detail::detacher,
        crate::routes::admin_list::changer_letat,
        crate::routes::admin_list::ecran_de_liste,
        crate::routes::admin_list::pilotage,
        crate::routes::admin_list::comite,
        crate::routes::admin_list::confier,
        crate::routes::admin_desk::fiche,
        crate::routes::admin_desk::noter,
        crate::routes::admin_desk::se_deporter,
        crate::routes::admin_desk::decider,
        crate::routes::workspace::espace,
        crate::routes::workspace::editions,
        crate::routes::workspace::fichier,
        crate::routes::workspace::ecrire,
        crate::routes::workspace::resoudre,
        crate::routes::workspace::rouvrir,
        crate::routes::admin_ops::deduire,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Dépôt", description = "Côté organisation : le brouillon, son enregistrement automatique, le dépôt et le renvoi. Gardé par l'ADHÉSION ACTIVE à l'organisation porteuse, pas par un périmètre d'administration — une organisation n'administre rien."),
        (name = "Espace organisation", description = "Ce qu'une organisation voit de ses propres dossiers. Ni note, ni rang, ni nom de membre du comité, ni inscrit nommé : le filtrage est à la source, jamais dans l'écran."),
        (name = "Back-office — propositions", description = "La liste, la fiche d'évaluation, les décisions. Permission de lecture générale ET périmètre d'administration : un dossier remonte à son édition AVANT que le périmètre soit vérifié, y compris sur une URL forgée."),
        (name = "Back-office — évaluation", description = "Notation, déport, messages. Noter exige une AFFECTATION non déportée ; lire n'en exige pas — la permission et l'affectation sont décorrélées."),
    )
)]
pub struct ProgrammeApi;

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    /// **Les trente-sept routes du contrat sont documentées.**
    ///
    /// Le compte est écrit, comme celui du test de montage : une route ajoutée
    /// sans annotation n'apparaîtrait nulle part, et une annotation posée sur
    /// une route non montée ferait rendre 404 à la documentation.
    ///
    /// Les chemins sont comptés, pas les opérations : `/proposals/{id}/documents`
    /// en porte deux — une lecture et un rattachement —, et
    /// `/proposal-comments/{id}/resolution` aussi.
    #[test]
    fn les_trente_sept_routes_sont_documentees() {
        let doc = super::ProgrammeApi::openapi();
        let operations: usize = doc
            .paths
            .paths
            .values()
            .map(|chemin| {
                [
                    chemin.get.is_some(),
                    chemin.put.is_some(),
                    chemin.post.is_some(),
                    chemin.delete.is_some(),
                    chemin.patch.is_some(),
                ]
                .iter()
                .filter(|servi| **servi)
                .count()
            })
            .sum();

        assert_eq!(operations, 37, "les trente-sept routes du contrat");
    }

    /// **Les six codes du module sont au catalogue du noyau.** Ils sont
    /// engendrés dans la documentation depuis lui : un code ajouté apparaît au
    /// prochain démarrage, un code oublié n'existe pas.
    #[test]
    fn les_six_codes_sont_au_catalogue() {
        let catalogue: Vec<&str> = kernel::error::ErrorCode::ALL
            .iter()
            .map(|c| c.as_str())
            .collect();

        for code in [
            "PROPOSAL_NOT_EDITABLE",
            "PROPOSAL_SPEAKER_IDENTITY_LOCKED",
            "PROPOSAL_REVIEW_NOT_ASSIGNED",
            "PROPOSAL_UNKNOWN_TERM",
            "PROPOSAL_TEXT_TOO_LONG",
            "PROPOSAL_UNKNOWN_REFERENCE",
        ] {
            assert!(catalogue.contains(&code), "{code} manque au catalogue");
        }
    }
}
