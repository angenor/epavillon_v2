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
        crate::routes::acces::demander,
        crate::routes::acces::annuler_sa_demande,
        crate::routes::themes::mes_thematiques,
        crate::routes::themes::suivre_des_thematiques,
        crate::routes::admin_codes::lister,
        crate::routes::admin_codes::creer,
        crate::routes::admin_codes::fiche,
        crate::routes::admin_codes::revoquer,
        crate::routes::admin_codes::usages,
        crate::routes::admin_codes::retirer_un_acces,
        crate::routes::admin_codes::retirer_tous_les_acces,
        crate::routes::admin_requests::file,
        crate::routes::admin_requests::admettre,
        crate::routes::admin_requests::refuser,
        crate::routes::admin_admission::lire,
        crate::routes::admin_admission::ecrire,
        crate::routes::documents::bibliotheque,
        crate::routes::documents::notes,
        crate::routes::documents::lecture,
        crate::routes::documents::fichier,
        crate::routes::documents::telecharge,
        crate::routes::documents::favoris,
        crate::routes::documents::poser_un_favori,
        crate::routes::documents::retirer_un_favori,
        crate::routes::savoir::paquet,
        crate::routes::savoir::favoris,
        crate::routes::savoir::poser_un_favori,
        crate::routes::savoir::retirer_un_favori,
        crate::routes::savoir::lire_une_entree,
        crate::routes::admin_documents::lister,
        crate::routes::admin_documents::creer,
        crate::routes::admin_documents::fiche,
        crate::routes::admin_documents::modifier,
        crate::routes::admin_documents::supprimer,
        crate::routes::admin_documents::attacher_le_fichier,
        crate::routes::admin_documents::pdf,
        crate::routes::admin_documents::relancer,
        crate::routes::admin_documents::texte_agrandi,
        crate::routes::admin_documents::publier,
        crate::routes::admin_documents::depublier,
        crate::routes::admin_documents::nouvelle_version,
        crate::routes::admin_documents::apercu,
        crate::routes::admin_documents::image,
        crate::routes::admin_documents::notes,
        crate::routes::admin_documents::poser_une_note,
        crate::routes::admin_documents::retirer_une_note,
        crate::routes::admin_savoir::liste_faq,
        crate::routes::admin_savoir::creer_faq,
        crate::routes::admin_savoir::fiche_faq,
        crate::routes::admin_savoir::modifier_faq,
        crate::routes::admin_savoir::supprimer_faq,
        crate::routes::admin_savoir::verifier_faq,
        crate::routes::admin_savoir::publier_faq,
        crate::routes::admin_savoir::faq_a_revoir,
        crate::routes::admin_savoir::depublier_faq,
        crate::routes::admin_savoir::liste_lexique,
        crate::routes::admin_savoir::creer_terme,
        crate::routes::admin_savoir::fiche_terme,
        crate::routes::admin_savoir::modifier_terme,
        crate::routes::admin_savoir::supprimer_terme,
        crate::routes::admin_savoir::publier_terme,
        crate::routes::admin_savoir::terme_a_revoir,
        crate::routes::admin_savoir::depublier_terme,
        crate::routes::admin_savoir::parcours,
        crate::routes::admin_savoir::creer_groupe,
        crate::routes::admin_savoir::modifier_groupe,
        crate::routes::admin_savoir::supprimer_groupe,
        crate::routes::admin_savoir::creer_etape,
        crate::routes::admin_savoir::modifier_etape,
        crate::routes::admin_savoir::supprimer_etape,
        crate::routes::admin_savoir::ordonner,
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Guide Négo — accès", description = "Entrer dans les modules réservés : le code d'invitation, l'état de son propre accès, la demande à trancher."),
        (name = "Guide Négo — documents", description = "La bibliothèque de documents, leur forme lisible, les images de page, les notes de correction, le compteur et les favoris. Lecture ouverte à tous ; un document réservé ne livre son contenu qu'à qui a l'accès négociateur."),
        (name = "Guide Négo — savoir", description = "La FAQ, le parcours « Ma première COP » et le lexique, en un paquet que le téléphone garde et relit par différence. Lecture ouverte à tous ; les termes favoris demandent une session."),
        (name = "Back-office — documents", description = "Publier un document en une journée : brouillon, PDF, extraction, aperçu page par page, publication, nouvelle version ; et les notes de correction de l'expert. Portée globale."),
        (name = "Back-office — savoir", description = "La FAQ, le lexique et le parcours « Ma première COP » : rédiger, dater la vérification (l'expert seul), publier, mettre « À revoir », dépublier, ordonner. Portée globale ; lecture ouverte à qui publie ou vérifie."),
        (name = "Back-office — admission", description = "Les codes d'invitation, leurs usages, les demandes d'accès et le mode d'admission. Réservé aux administrateurs de la plateforme entière : `negotiation.space.manage` **sur la portée globale**. Un administrateur d'une seule édition n'y voit rien — aucun espace de négociation n'est rattaché à un événement."),
    )
)]
pub struct NegotiationApi;
