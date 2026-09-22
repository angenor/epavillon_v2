//! Le mode d'admission : le lire, le basculer.
//!
//! Deux routes, même garde que le reste du back-office de Guide Négo :
//! **`Requires<SpaceManage>` sur la portée globale**. Le mode commande l'entrée
//! de toute l'application ; il n'appartient à aucune édition.
//!
//! **La bascule prend effet à la tentative suivante**, sans mise en ligne ni
//! redémarrage : la valeur est relue à chaque saisie de code, sans cache. C'est
//! ce que SC-002 mesure.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;

use crate::domain::admission::UpdateAdmissionModePayload;
use crate::domain::permissions::SpaceManage;
use crate::service::admission;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/admission", web::get().to(lire))
        .route("/admin/negotiation/admission", web::put().to(ecrire));
}

#[utoipa::path(
    get,
    description = "`AdmissionSettings` — le mode d'admission courant, et les trois valeurs offertes.\n\nChaque option porte ce qu'elle produit **en faits** : la saisie d'un code est-elle proposée (`offers_code`), un code juste ouvre-t-il aussitôt (`code_opens`), un administrateur doit-il trancher (`needs_approval`). L'écran compose sa phrase à partir de là, par ses fichiers de traduction, comme tout écran du site : rendre ici du texte français donnerait deux catalogues pour un même écran.",
    path = "/admin/negotiation/admission",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_admission_lire",
    responses(
        (status = 200, description = "AdmissionSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.space.manage` **sur la portée globale**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire(
    state: web::Data<NegotiationState>,
    _garde: Requires<SpaceManage>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(admission::lire(&state).await?))
}

#[utoipa::path(
    put,
    description = "`UpdateAdmissionModePayload` → `AdmissionSettings` — bascule le mode d'admission.\n\n**Prend effet à la tentative suivante**, sans mise en ligne : la valeur vit dans `platform.settings` et se relit à chaque saisie de code, sans cache.\n\nUne demande déjà en attente **survit à la bascule** et reste traitable (FR-029) : changer le mode ne touche à aucune demande.\n\nUne valeur hors des trois sort en `NEGOTIATION_ADMISSION_MODE_INVALID`, qui désigne le champ `mode` — l'écran la pose sous le sélecteur, pas en bandeau de panne.",
    path = "/admin/negotiation/admission",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_admission_ecrire",
    request_body = Object,
    responses(
        (status = 200, description = "AdmissionSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Mode inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    charge: web::Json<UpdateAdmissionModePayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reglage = admission::ecrire(&state, &contexte, garde.person_id, &charge.mode).await?;

    Ok(HttpResponse::Ok().json(reglage))
}
