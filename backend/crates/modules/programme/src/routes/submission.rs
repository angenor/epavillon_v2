//! Le dépôt, côté organisation.
//!
//! # L'autorisation n'est pas celle du back-office
//!
//! Ces routes sont gardées par la permission de **soumettre** — détenue sans
//! portée par les membres d'organisation — **et** par l'**adhésion active** à
//! l'organisation porteuse, qui est le vrai contrôle. Une organisation
//! n'administre rien : le périmètre d'administration n'a pas cours ici (R13).
//!
//! # Les chemins littéraux précèdent le chemin paramétré
//!
//! `/proposals/form-context` et `/proposals/draft` sont déclarés avant
//! `/proposals/{id}` : sans cela, « form-context » serait lu comme un
//! identifiant de dossier. C'est le même avertissement qu'en B3.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Actor, RequiresAnyScope};
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::draft::SaveDraftPayload;
use crate::domain::ids::ProposalId;
use crate::domain::permissions::ProposalSubmit;
use crate::repo::{cross, proposals};
use crate::routes::contexte_de;
use crate::service::{draft_write, submit};
use crate::state::ProgrammeState;

/// Les organisations dont l'écran demande le décompte, telles que le front les
/// envoie : une liste séparée par des virgules.
#[derive(Debug, Deserialize)]
pub struct OrganisationsDemandees {
    #[serde(default)]
    organization_ids: Option<String>,
}

/// Ce que le contexte du formulaire rend, avec le brouillon en cours.
#[derive(Debug, serde::Serialize)]
struct BrouillonEnCours {
    #[serde(flatten)]
    enregistrement: proposals::Enregistrement,
}

/// Ce que ce fichier dépose **sous un chemin littéral**. Voir `lib.rs` : tous
/// les littéraux du scope sont enregistrés avant tout chemin paramétré.
pub fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("/form-context", web::get().to(contexte_du_formulaire))
        .route("/draft", web::get().to(mon_brouillon))
        .route("", web::post().to(creer));
}

/// Ce que ce fichier dépose **sous un identifiant de dossier**.
pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}", web::put().to(modifier))
        .route("/{id}/draft", web::get().to(rouvrir))
        .route("/{id}/submit", web::post().to(deposer))
        .route("/{id}/resubmit", web::post().to(renvoyer));
}

/// Rouvrir un dossier tel qu'il a été saisi.
#[utoipa::path(
    get,
    description = "`EditableProposal` — le dossier **recomposé en brouillon**, pas un `SELECT`. Trois conversions comptent : le créneau redevient une **heure murale dans le fuseau de l'ÉDITION** — saisi à 14:30 à Belém, il se rouvrirait à 11:30 pour qui corrige depuis Dakar, sans qu'aucune erreur ne soit levée ; les textes multilingues sont ramenés à leur français, **les textes provisoires effacés** — le formulaire n'affiche jamais « Dossier sans titre » (écart n° 102) ; chaque intervenant retrouve son **verrouillage d'identité** — une personne qui possède un compte détient sa fiche (écart n° 31). **Une seule implémentation pour les deux écrans** : deux recompositions divergeraient au premier champ ajouté.",
    path = "/proposals/{id}/draft",
    tag = "Dépôt",
    operation_id = "depot_rouvrir_un_dossier",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "EditableProposal", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rouvrir(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let dossier =
        crate::service::draft_read::rouvrir(&state, acteur.0, ProposalId(chemin.into_inner()))
            .await?;

    Ok(HttpResponse::Ok().json(dossier))
}

/// Renvoyer un dossier corrigé.
#[utoipa::path(
    post,
    description = "`SaveDraftPayload` → `SubmitProposalResult`. **Ce n'est pas un dépôt** (écart n° 38) : la **fenêtre de l'appel ne s'applique pas** — le comité demande une correction à huit jours de la clôture, l'organisation répond après l'échéance, et lui opposer la clôture serait lui reprocher un délai qu'elle n'a pas choisi. Le déclencheur du modèle le sait déjà : il ne vérifie la fenêtre qu'au premier dépôt. **Le plafond, lui, s'applique** : il compte les dossiers en course, et un renvoi en remet un. Le geste est porté par le **chemin**, jamais déduit de l'état — déduire ferait franchir la clôture à un dossier corrigé par la route de dépôt, sans que personne l'ait décidé.",
    path = "/proposals/{id}/resubmit",
    tag = "Dépôt",
    operation_id = "depot_renvoyer_un_dossier",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "SubmitProposalResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou porté par une organisation dont on n'est pas membre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Le dossier n'attend aucune correction, ou l'édition est terminée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn renvoyer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: RequiresAnyScope<ProposalSubmit>,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveDraftPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.person_id);
    let resultat = crate::service::resubmit::renvoyer(
        &state,
        &ctx,
        acteur.person_id,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Où l'on dépose aujourd'hui, et ce que l'organisation a déjà déposé.
#[utoipa::path(
    get,
    description = "`ProposalFormContext` — l'appel **réellement ouvert** de la plateforme (statut ET fenêtre, par `event.is_call_open()`), son édition, et le décompte du plafond de l'organisation, **ce brouillon exclu**. Le formulaire ne choisit pas son édition : il y en a au plus une qui reçoit. Le décompte reprend exactement les trois états que le déclencheur de recevabilité écarte — brouillon, retiré, non retenu —, sans quoi l'écran annoncerait un plafond que la base ne tient pas. Rend des champs nuls quand aucun appel n'est ouvert : l'écran l'annonce et s'arrête.",
    path = "/proposals/form-context",
    tag = "Dépôt",
    operation_id = "depot_contexte_du_formulaire",
    params(("organization_ids" = Option<String>, Query, description = "Organisations de la personne, séparées par des virgules")),
    responses(
        (status = 200, description = "ProposalFormContext", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn contexte_du_formulaire(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    demande: web::Query<OrganisationsDemandees>,
) -> Result<HttpResponse> {
    // **Les organisations reçues sont recoupées avec les adhésions actives.**
    // Un client qui enverrait l'identifiant d'une organisation dont il n'est
    // pas membre lirait sinon son décompte de dossiers — mince, mais c'est une
    // fuite, et la lecture qui la referme existe déjà.
    let siennes = cross::organisations_actives(state.pool(), acteur.0).await?;
    let demandees = decouper(demande.organization_ids.as_deref());
    let retenues: Vec<Uuid> = if demandees.is_empty() {
        siennes
    } else {
        demandees
            .into_iter()
            .filter(|o| siennes.contains(o))
            .collect()
    };

    let brouillon = proposals::brouillon_en_cours(state.pool(), acteur.0).await?;
    let contexte = proposals::contexte_du_formulaire(
        state.pool(),
        &retenues,
        brouillon.as_ref().map(|b| ProposalId(b.proposal_id)),
    )
    .await?;

    Ok(HttpResponse::Ok().json(contexte))
}

/// Le brouillon en cours de la personne, pour reprendre où elle s'est arrêtée.
#[utoipa::path(
    get,
    description = "`SaveDraftResult`, ou `null`. Le **plus récent** des brouillons de la personne : rien n'interdit d'en avoir deux — un par organisation —, et le contrat n'en rend qu'un. Ne rend jamais un dossier déposé : reprendre un dossier existant passe par la route de recomposition.",
    path = "/proposals/draft",
    tag = "Dépôt",
    operation_id = "depot_mon_brouillon",
    responses(
        (status = 200, description = "SaveDraftResult ou null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn mon_brouillon(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
) -> Result<HttpResponse> {
    let brouillon = proposals::brouillon_en_cours(state.pool(), acteur.0).await?;

    Ok(
        HttpResponse::Ok()
            .json(brouillon.map(|enregistrement| BrouillonEnCours { enregistrement })),
    )
}

/// Le premier enregistrement — **celui qui crée la ligne et attribue le numéro**.
#[utoipa::path(
    post,
    description = "`SaveDraftPayload` → `SaveDraftResult`. **Le dossier naît toujours en brouillon**, quel que soit l'état demandé : le garde d'état n'est posé que sur la mise à jour de `status`, et une insertion lui échappe (écart n° 96). Le numéro de dossier est attribué **à l'insertion** par le déclencheur, et l'écran peut donc l'annoncer dès la première frappe — c'est le même qui figurera sur la confirmation de dépôt. L'adresse d'URL est **dérivée par le service**, repliée quand le titre est vide et suffixée sur collision : le contrat ne la porte pas, et sans elle le tout premier enregistrement échouerait (écart n° 95).",
    path = "/proposals",
    tag = "Dépôt",
    operation_id = "depot_creer_brouillon",
    request_body = Object,
    responses(
        (status = 200, description = "SaveDraftResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de soumettre absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Adhésion inactive à l'organisation porteuse — indiscernable d'un dossier inexistant", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte trop long, thématique inconnue, identité verrouillée, bornes de l'appel", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    droit: RequiresAnyScope<ProposalSubmit>,
    corps: web::Json<SaveDraftPayload>,
) -> Result<HttpResponse> {
    let payload = SaveDraftPayload {
        // Une création ne reprend jamais un identifiant du corps : la route dit
        // ce qui se passe, pas la charge utile.
        proposal_id: None,
        ..corps.into_inner()
    };
    let ctx = contexte_de(&requete, droit.person_id);
    let ligne = draft_write::enregistrer(&state, &ctx, droit.person_id, payload).await?;

    Ok(HttpResponse::Ok().json(ligne))
}

/// Les enregistrements suivants — **sans jamais toucher à l'état**.
#[utoipa::path(
    put,
    description = "`SaveDraftPayload` → `SaveDraftResult`. **Corriger n'est pas déposer** : `status` n'est pas dans la mise à jour, et le garde d'état n'est donc pas réveillé — un dossier en évaluation ne repart pas au comité parce qu'on a rectifié une faute de frappe. L'adresse d'URL **suit le titre tant que le dossier est en brouillon**, et se fige au dépôt : une adresse déjà communiquée ne change pas sous une correction. L'organisation porteuse vient de **la base**, jamais du corps.",
    path = "/proposals/{id}",
    tag = "Dépôt",
    operation_id = "depot_enregistrer_brouillon",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "SaveDraftResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de soumettre absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou porté par une organisation dont vous n'êtes pas membre actif** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Dossier clos (PROPOSAL_NOT_EDITABLE), texte trop long, thématique inconnue, identité verrouillée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    droit: RequiresAnyScope<ProposalSubmit>,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveDraftPayload>,
) -> Result<HttpResponse> {
    let payload = SaveDraftPayload {
        proposal_id: Some(chemin.into_inner()),
        ..corps.into_inner()
    };
    let ctx = contexte_de(&requete, droit.person_id);
    let ligne = draft_write::enregistrer(&state, &ctx, droit.person_id, payload).await?;

    Ok(HttpResponse::Ok().json(ligne))
}

/// Le dépôt.
#[utoipa::path(
    post,
    description = "`SaveDraftPayload` → `SubmitProposalResult`. **Les trois refus de recevabilité sortent en 200**, avec leur valeur : l'échéance pour un appel clos, le plafond pour un quota atteint. Ils sont classés **avant** l'écriture parce que le déclencheur ne les rend que dans une phrase française, et parce qu'un même code d'erreur PostgreSQL sert aux quatre causes possibles. Le déclencheur reste le dernier mot : une course est **reclassée**, jamais lue au texte. Le brouillon est enregistré **avant** toute décision — si l'appel a fermé entre le chargement et le clic, l'organisation ne perd pas en plus ce qu'elle venait de saisir. La réponse porte le nombre de revues attendues et la date d'annonce, **lus sur l'appel**.",
    path = "/proposals/{id}/submit",
    tag = "Dépôt",
    operation_id = "depot_deposer",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "SubmitProposalResult — submitted, call_closed, quota_reached", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de soumettre absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors de vos organisations** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Dossier incomplet, ou bornes d'intervenants de l'appel", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn deposer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    droit: RequiresAnyScope<ProposalSubmit>,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveDraftPayload>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    let ctx = contexte_de(&requete, droit.person_id);
    let resultat =
        submit::deposer(&state, &ctx, droit.person_id, dossier, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Une liste d'identifiants séparés par des virgules, telle que le front
/// l'écrit. Ce qui n'est pas un identifiant est **ignoré** plutôt que refusé :
/// le paramètre est un filtre de confort, pas une clé.
fn decouper(valeur: Option<&str>) -> Vec<Uuid> {
    valeur
        .unwrap_or_default()
        .split(',')
        .filter_map(|part| Uuid::parse_str(part.trim()).ok())
        .collect()
}
