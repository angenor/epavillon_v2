//! Les actions groupées de la liste du back-office.
//!
//! # Le périmètre est exigé, et il est vérifié DEUX fois
//!
//! L'extracteur refuse un périmètre **vide** — un refus explicite, jamais une
//! liste vide : les confondre afficherait « rien à traiter » à qui n'a aucun
//! droit. Puis le service vérifie **dossier par dossier**, parce qu'une
//! sélection de douze peut traverser deux éditions.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::routes::contexte_de;
use crate::service::list::{self, AssignReviewerPayload};
use crate::service::transition::{self, ChangeStatusPayload};
use crate::state::ProgrammeState;

/// L'édition demandée. **Elle est exigée** : ces écrans travaillent sur une
/// édition, et rendre « tout le périmètre » à défaut mélangerait deux COP dans
/// un même tableau de classement.
#[derive(Debug, Deserialize)]
pub struct EditionDemandee {
    event_id: Uuid,
}

/// Ce que ce fichier dépose sous un chemin littéral.
///
/// **Tous précèdent `/proposals/{id}`**, et depuis US3 le risque est réel : ce
/// chemin est désormais servi en `GET`, comme trois des quatre ci-dessous.
pub fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("/list", web::get().to(ecran_de_liste))
        .route("/dashboard", web::get().to(pilotage))
        .route("/committee", web::get().to(comite))
        .route("/assignments", web::post().to(confier))
        .route("/status", web::post().to(changer_letat));
}

/// Tout l'écran de la liste, en une réponse.
#[utoipa::path(
    get,
    description = "`ProposalListScreen` — les lignes de `programme.v_proposal_dashboard`, **les sept facettes comptées sur ces mêmes lignes**, les dossiers que la personne connectée n'a jamais ouverts, le fuseau de l'édition, sa ville, l'échéance effective de l'appel et le nombre de revues attendues. Demandées à part, les facettes seraient mesurées à un autre instant et le « Retenu (17) » du filtre finirait par ne plus correspondre aux lignes affichées. **Ni pagination, ni tri, ni filtre serveur** : le contrat du front les garde à l'écran jusqu'au raccordement. **Périmètre vide → refus explicite**, jamais une liste vide ; **édition hors périmètre → le même refus qu'une édition inexistante**.",
    path = "/proposals/list",
    tag = "Back-office — propositions",
    operation_id = "propositions_ecran_de_liste",
    params(("event_id" = Uuid, Query, description = "Édition dont on liste les dossiers")),
    responses(
        (status = 200, description = "ProposalListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ecran_de_liste(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let ecran = list::ecran(&state, &perimetre, EventId(demande.event_id)).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// Les lignes seules.
#[utoipa::path(
    get,
    description = "`ProposalDashboardRow[]` — la vue de pilotage **telle quelle**, sans facettes ni non-lus. Le titre y voyage **deux fois** : `title`, document multilingue brut résolu à l'affichage, et `title_text`, sa résolution française réservée au tri, au filtrage et à l'export. Les confondre rendrait une chaîne vide sans erreur. Les dossiers effacés sont exclus par la vue.",
    path = "/proposals/dashboard",
    tag = "Back-office — propositions",
    operation_id = "propositions_pilotage",
    params(("event_id" = Uuid, Query, description = "Édition dont on liste les dossiers")),
    responses(
        (status = 200, description = "ProposalDashboardRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn pilotage(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let lignes = list::lignes(&state, &perimetre, EventId(demande.event_id)).await?;
    Ok(HttpResponse::Ok().json(lignes))
}

/// Qui peut recevoir une affectation, et ce qu'il porte déjà.
#[utoipa::path(
    get,
    description = "`ProposalFacet[]` — la composition du comité de l'appel : la valeur est la personne, le libellé son nom, **le décompte sa charge courante** sur cet appel, déports exclus. On ne confie pas douze dossiers de plus à quelqu'un qui en porte déjà vingt. Une édition sans appel rend une liste vide : il n'y a alors aucun comité, ce qui est un fait et non une erreur.",
    path = "/proposals/committee",
    tag = "Back-office — propositions",
    operation_id = "propositions_comite",
    params(("event_id" = Uuid, Query, description = "Édition dont on lit le comité")),
    responses(
        (status = 200, description = "ProposalFacet[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn comite(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let membres = list::comite(&state, &perimetre, EventId(demande.event_id)).await?;
    Ok(HttpResponse::Ok().json(membres))
}

/// Confier une sélection à un membre du comité.
#[utoipa::path(
    post,
    description = "`AssignReviewerPayload` → `BulkResult`. Gardé par `event.call.manage` **sur l'édition de chaque dossier** — composer le comité et répartir sa charge sont le même geste, celui de qui tient la campagne (écart n° 48). Trois écarts nommés : **déjà confié**, **déporté** — le lui réattribuer effacerait une déclaration d'impartialité —, **introuvable**, qui couvre aussi le hors-périmètre et le hors-permission sans les distinguer. **Un événement `programme.review.assigned` par dossier**, jamais un pour le lot : un consommateur qui reçoit un lot doit le déplier lui-même, et son échec porterait alors sur douze effets au lieu d'un.",
    path = "/proposals/assignments",
    tag = "Back-office — propositions",
    operation_id = "propositions_confier_en_groupe",
    request_body = Object,
    responses(
        (status = 200, description = "BulkResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Membre du comité inconnu — PROPOSAL_UNKNOWN_REFERENCE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn confier(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    corps: web::Json<AssignReviewerPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = list::confier_en_groupe(&state, &ctx, &perimetre, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Changer l'état d'une sélection.
#[utoipa::path(
    post,
    description = "`ChangeStatusPayload` → `BulkResult`. **L'autorisation est évaluée dossier par dossier** : une sélection peut traverser deux éditions, et le périmètre s'applique à chacune. Chaque dossier qui n'a pas suivi ressort avec son numéro et sa raison — transition non offerte, motif manquant, introuvable. Répondre « 6 dossiers traités » sans dire ce qu'il est advenu des six autres serait le défaut classique des actions de masse. **Un dossier hors périmètre rend le même écart qu'un dossier inexistant** : le refus ne dit pas à qui forge une sélection que le dossier existe ailleurs. Aucun événement n'est émis par le service : le déclencheur d'état les émet déjà, **un par dossier**.",
    path = "/proposals/status",
    tag = "Back-office — propositions",
    operation_id = "propositions_changer_letat_en_groupe",
    request_body = Object,
    responses(
        (status = 200, description = "BulkResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide — refus explicite, jamais une liste vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn changer_letat(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    corps: web::Json<ChangeStatusPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        transition::changer_en_groupe(&state, &ctx, &perimetre, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
