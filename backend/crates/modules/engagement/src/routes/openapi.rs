//! Documentation OpenAPI du module — **engendrée**, jamais écrite à la main.

use utoipa::{OpenApi, ToSchema};

/// Forme du corps d'erreur, référencée par chaque route.
#[derive(ToSchema)]
#[schema(as = ApiError)]
#[allow(dead_code)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub request_id: Option<String>,
}

/// Les chemins s'ajoutent ici **au fil des histoires**, jamais d'avance.
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::rules::lister,
        crate::routes::rules::ecrire,
        crate::routes::rules::supprimer,
        crate::routes::sessions::calendrier,
        crate::routes::sessions::regle_applicable,
        crate::routes::templates::lister,
        crate::routes::templates::detail,
        crate::routes::templates::ecrire_revision,
        crate::routes::templates::publier,
        crate::routes::templates::apercu,
        crate::routes::notifications::lire,
        crate::routes::notifications::marquer_lues,
        crate::routes::notifications::archiver,
        crate::routes::preferences::lire,
        crate::routes::preferences::ecrire,
        crate::routes::suppressions::lister,
        crate::routes::suppressions::poser,
        crate::routes::suppressions::retirer,
        crate::routes::broadcast::diffuser,
        crate::routes::internal::ingerer,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Rappels — calendrier", description = "Ce qu'une organisation voit des rappels de sa séance : **une ligne par décalage et par canal**, avec un NOMBRE de destinataires et jamais un nom. `has_rule` distingue « aucune règle » de « tout est parti » — une liste vide muette se confond avec un envoi réussi. Gardé par l'ADHÉSION ACTIVE, pas par un périmètre : une organisation n'administre rien."),
        (name = "Back-office — règles de rappel", description = "La politique : une LISTE de décalages cumulés, jamais un décalage seul. Une règle de séance remplace celle de son édition, sans cumul — et la lecture rend l'ORIGINE de la règle applicable, ce qui rend la non-cumulation vérifiable de l'extérieur."),
        (name = "Notifications", description = "La liste et le compte de non lues **dans la même réponse** : deux appels donneraient deux chiffres mesurés à deux instants. Une préférence posée sur un type critique est enregistrée telle quelle, et la lecture DIT qu'elle n'oppose rien."),
        (name = "Back-office — modèles de messages", description = "Révisions, publication et retour arrière. Le corps est **assaini à l'écriture**, avec une liste blanche propre au courriel — tableaux et styles en ligne, que les clients de messagerie exigent. Un type sans révision publiée part quand même, avec un texte de secours qui le dit."),
        (name = "Délivrabilité", description = "La liste de suppression, et l'ingestion des retours du fournisseur. Cette dernière est **hors session**, authentifiée par un jeton porteur — et **non montée** quand le jeton n'est pas configuré : une porte d'ingestion sans secret vaut mieux fermée."),
    )
)]
pub struct EngagementApi;
