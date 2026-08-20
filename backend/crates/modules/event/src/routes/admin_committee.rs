//! Le comité de sélection d'un appel.
//!
//! **Gardé par la permission de gestion des APPELS**, et non par celle des
//! événements : le comité appartient à l'appel, comme sa grille. Un compte peut
//! tenir le décor d'une édition — ses salles, ses journées — sans avoir à
//! composer son comité de sélection.
//!
//! **Ajouter quelqu'un n'accorde aucun droit.** La réponse porte
//! `has_review_permission` et se contente de le dire ; l'autorisation reste
//! portée par les attributions de rôle, sur la portée de l'édition.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::CallId;
use crate::domain::tabs::CommitteePayload;
use crate::routes::contexte_de;
use crate::service::{committee as service_comite, edition_dans_le_perimetre, Cible};
use crate::state::EventState;

/// Enregistrer la composition — **ajouts, retraits et plafonds d'un seul
/// geste**.
#[utoipa::path(
    put,
    description = "`CommitteePayload` → `CommitteeSaveResult`. **Ajouts, retraits et plafonds d'un seul geste**, dans une transaction : l'écran envoie la liste complète, et ce qui n'y figure plus est retiré. Les doublons de charge utile sont **dédoublonnés par le service**, jamais remontés comme erreur de base. Une personne inconnue rend `EVENT_UNKNOWN_REFERENCE` en 422, **en la nommant** — la clé étrangère refuserait aussi, mais sans dire laquelle des lignes est en cause. `removed_with_assignments` nomme les membres retirés portant encore des dossiers : leurs revues rendues restent au dossier, mais quelqu'un doit reprendre le reste. **Siéger n'accorde aucun droit** : `has_review_permission` le dit, il ne le donne pas.",
    path = "/admin/calls/{id}/reviewers",
    tag = "Back-office — appel à propositions",
    operation_id = "admin_comite_enregistrer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'appel")),
    request_body = Object,
    responses(
        (status = 200, description = "CommitteeSaveResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Appel inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Personne inconnue (EVENT_UNKNOWN_REFERENCE)", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn enregistrer(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<CommitteePayload>,
) -> Result<HttpResponse> {
    let call_id = CallId::from(chemin.into_inner());
    let event_id =
        edition_dans_le_perimetre(state.pool(), &perimetre, Cible::Appel(call_id)).await?;

    super::admin_call::autoriser(&state, &perimetre, event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_comite::enregistrer(&state, &ctx, event_id, call_id, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
