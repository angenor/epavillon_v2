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
    ),
    components(schemas(ApiErrorBody)),
    tags(
        (name = "Guide Négo — accès", description = "Entrer dans les modules réservés : le code d'invitation, l'état de son propre accès, la demande à trancher."),
        (name = "Back-office — admission", description = "Les codes d'invitation, leurs usages, les demandes d'accès et le mode d'admission. Réservé aux administrateurs de la plateforme entière : `negotiation.space.manage` **sur la portée globale**. Un administrateur d'une seule édition n'y voit rien — aucun espace de négociation n'est rattaché à un événement."),
    )
)]
pub struct NegotiationApi;
