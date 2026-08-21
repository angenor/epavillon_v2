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
        // --- Séances et inscriptions (B5) : dix-sept chemins ------------------
        crate::routes::planner::ecran,
        crate::routes::sessions::liste,
        crate::routes::sessions::conflits,
        crate::routes::sessions::intervenants,
        crate::routes::sessions::organisations,
        crate::routes::sessions::fils,
        crate::routes::sessions::placer,
        crate::routes::sessions::rattacher,
        crate::routes::sessions::diffuser,
        crate::routes::public_schedule::programmation,
        crate::routes::public_schedule::seance,
        crate::routes::registrations::formulaire,
        crate::routes::registrations::sinscrire,
        crate::routes::registrations::liste_nominative,
        crate::routes::registrations::les_miennes,
        crate::routes::registrations::annuler,
        crate::routes::registrations::rejoindre,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Dépôt", description = "Côté organisation : le brouillon, son enregistrement automatique, le dépôt et le renvoi. Gardé par l'ADHÉSION ACTIVE à l'organisation porteuse, pas par un périmètre d'administration — une organisation n'administre rien."),
        (name = "Espace organisation", description = "Ce qu'une organisation voit de ses propres dossiers. Ni note, ni rang, ni nom de membre du comité, ni inscrit nommé : le filtrage est à la source, jamais dans l'écran."),
        (name = "Back-office — propositions", description = "La liste, la fiche d'évaluation, les décisions. Permission de lecture générale ET périmètre d'administration : un dossier remonte à son édition AVANT que le périmètre soit vérifié, y compris sur une URL forgée."),
        (name = "Back-office — évaluation", description = "Notation, déport, messages. Noter exige une AFFECTATION non déportée ; lire n'en exige pas — la permission et l'affectation sont décorrélées."),
        (name = "Planificateur de séances", description = "L'écran d'arbitrage et ses trois écritures. **Aucune ne peut être refusée pour chevauchement** : le modèle ne pose aucune contrainte d'exclusion sur les créneaux, l'équipe travaille par déplacements successifs, et le seul garde-fou dur est la publication du programme. Gardé par `programme.session.schedule` sur l'édition DE LA SÉANCE, résolue en base."),
        (name = "Inscriptions", description = "Le formulaire applicable, l'inscription et ses six issues, l'annulation avec sa promotion. Trois régimes d'autorisation distincts : le formulaire décide si l'on peut s'inscrire sans compte, la liste nominative exige `programme.registration.manage` — que le rôle de programmation ne détient pas —, et l'annulation est ouverte à l'inscrit lui-même."),
        (name = "Programmation publique", description = "Ce que le public lit, **sans session**. Seules les séances publiées y figurent ; une édition dont le programme n'est pas paru rend une liste vide, jamais une erreur."),
    )
)]
pub struct ProgrammeApi;

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    /// **Les cinquante-quatre routes du contrat sont documentées** — trente-sept
    /// de B4, dix-sept de B5.
    ///
    /// Le compte est écrit, comme celui du test de montage : une route ajoutée
    /// sans annotation n'apparaîtrait nulle part, et une annotation posée sur
    /// une route non montée ferait rendre 404 à la documentation.
    ///
    /// Les chemins sont comptés, pas les opérations : `/proposals/{id}/documents`
    /// en porte deux — une lecture et un rattachement —, et
    /// `/proposal-comments/{id}/resolution` aussi.
    #[test]
    fn les_cinquante_quatre_routes_sont_documentees() {
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

        assert_eq!(operations, 54, "trente-sept routes de B4, dix-sept de B5");
    }

    /// **Les dix-sept chemins de B5**, comptés à part : le total ci-dessus
    /// bougerait sans qu'on sache lequel des deux jalons a changé.
    ///
    /// `/sessions/{id}/tracks` en porte **deux** — une lecture et un
    /// remplacement —, d'où dix-sept opérations sur seize chemins.
    #[test]
    fn les_dix_sept_routes_de_b5_sont_documentees() {
        let doc = super::ProgrammeApi::openapi();
        let chemins: Vec<&str> = doc
            .paths
            .paths
            .keys()
            .filter(|c| {
                c.starts_with("/sessions")
                    || c.starts_with("/registrations")
                    || c.starts_with("/schedule")
                    || c.starts_with("/events/")
                    || c.starts_with("/admin/planner")
            })
            .map(String::as_str)
            .collect();

        let operations: usize = chemins
            .iter()
            .map(|c| {
                let chemin = &doc.paths.paths[*c];
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

        assert_eq!(operations, 17, "les dix-sept routes de B5");
    }

    /// **Les quatorze codes du module sont au catalogue du noyau** — six de B4,
    /// huit de B5. Ils sont
    /// engendrés dans la documentation depuis lui : un code ajouté apparaît au
    /// prochain démarrage, un code oublié n'existe pas.
    #[test]
    fn les_quatorze_codes_sont_au_catalogue() {
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
            "SESSION_DERIVED_FIELD",
            "SESSION_UNKNOWN_REFERENCE",
            "SESSION_TRACK_EVENT_MISMATCH",
            "REGISTRATION_NOT_ACCEPTED",
            "REGISTRATION_ANSWER_INVALID",
            "REGISTRATION_CONSENT_REQUIRED",
            "REGISTRATION_ACCOUNT_REQUIRED",
            "REGISTRATION_LOCKED",
        ] {
            assert!(catalogue.contains(&code), "{code} manque au catalogue");
        }
    }
}
