//! La fiche d'évaluation et ses trois écritures.
//!
//! # Toutes ces routes portent un identifiant de dossier
//!
//! Elles vivent donc dans `chemins_de_dossier`, enregistré **après** les
//! chemins littéraux du scope. Depuis US3, `GET /proposals/{id}` est servi :
//! un littéral déclaré après lui serait capturé.
//!
//! # Le périmètre et la permission ne disent pas la même chose
//!
//! `Perimeter` garde la **portée** — quelles éditions cette personne
//! administre —, et chaque service vérifie ensuite la **permission** sur
//! l'édition du dossier. Lire la fiche exige la lecture générale ; noter exige
//! `programme.review.write` **et** une affectation non déportée ; décider exige
//! ce que la règle de transition nomme.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::domain::transitions::ProposalStatus;
use crate::repo::proposals::Fiche;
use crate::repo::transitions::LigneDeJournal;
use crate::routes::contexte_de;
use crate::service::review::{RecusalPayload, SaveReviewPayload};
use crate::service::{desk, review, transition};
use crate::state::ProgrammeState;

/// Les chemins portant un identifiant de dossier.
pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/review-desk", web::get().to(fiche))
        .route("/{id}/reviews", web::put().to(noter))
        .route("/{id}/recusal", web::post().to(se_deporter))
        .route("/{id}/decision", web::post().to(decider));
}

/// Toute la fiche, en une réponse.
#[utoipa::path(
    get,
    description = "`ReviewDeskScreen` — onze tables en une réponse : le dossier, son édition, son appel, sa grille, ses organisations avec leur historique de participation, ses intervenants, ses pièces, son journal, son historique champ par champ, l'avancement nominatif du comité, ma revue, les échanges que **ce** lecteur a le droit de voir, et les revues des pairs **quand j'ai le droit de les lire**. **Le voile de l'aveugle n'est pas un filtre** : quand il est baissé — appel en aveugle, lecteur affecté, sa revue non déposée —, la requête qui lit les revues des pairs **n'est pas exécutée**. Le décompte l'est : compter n'ancre pas, lire si. **Cette lecture écrit** : elle pose l'accusé de lecture, et `first_visit` dit l'état d'AVANT la visite.",
    path = "/proposals/{id}/review-desk",
    tag = "Back-office — évaluation",
    operation_id = "propositions_fiche_devaluation",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ReviewDeskScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fiche(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let ecran = desk::ouvrir(&state, &ctx, &perimetre, ProposalId(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(ecran))
}

/// Enregistrer ou déposer sa revue.
#[utoipa::path(
    put,
    description = "`SaveReviewPayload` → `SaveReviewResult`. **Noter exige une affectation non déportée** : rien ne lie la permission à l'affectation en base, et un membre du comité pourrait sinon noter n'importe quel dossier de son édition. Lire, en revanche, reste permis — les deux règles sont décorrélées. **Une note absente n'est pas une note à zéro** : zéro sur un critère éliminatoire disqualifie le dossier. **La consolidation est appelée dans la même transaction**, et les agrégats rendus sont **relus en base** — sans cet appel, le classement du comité serait faux sans qu'aucune erreur ne le signale.",
    path = "/proposals/{id}/reviews",
    tag = "Back-office — évaluation",
    operation_id = "propositions_noter",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "SaveReviewResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Noter sans affectation, ou après un déport — PROPOSAL_REVIEW_NOT_ASSIGNED", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Note au-dessus du maximum de son critère, ou critère étranger à la grille", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn noter(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveReviewPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = review::enregistrer(
        &state,
        &ctx,
        &perimetre,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Se déporter.
#[utoipa::path(
    post,
    description = "`RecusalPayload` → `ReviewAssignment`. **Le motif est obligatoire, et c'est le sujet** : la colonne existe pour tracer l'impartialité du comité, et un déport sans motif ne se relit pas six mois plus tard, quand une organisation conteste. **Le déport n'efface pas l'affectation, il la date** : la ligne demeure, et c'est elle qui interdit une réattribution silencieuse. Une seconde demande sur une affectation déjà déportée rend la même ligne, sans rien réécrire.",
    path = "/proposals/{id}/recusal",
    tag = "Back-office — évaluation",
    operation_id = "propositions_se_deporter",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ReviewAssignment", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Aucune affectation à quitter — PROPOSAL_REVIEW_NOT_ASSIGNED", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Motif manquant", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn se_deporter(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<RecusalPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let affectation = review::se_deporter(
        &state,
        &ctx,
        &perimetre,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(affectation))
}

/// `DecisionPayload`.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct DecisionPayload {
    pub to_status: ProposalStatus,
    #[serde(default)]
    pub reason: Option<String>,
}

/// `DecisionResult` — **les deux refus sortent en 200**, le contrat les exprime
/// comme membres d'union.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DecisionResult {
    Applied {
        proposal: Box<Fiche>,
        transition: LigneDeJournal,
    },
    TransitionNotAllowed,
    ReasonRequired,
}

/// Décider — retenir, rejeter, remettre en évaluation, annuler.
#[utoipa::path(
    post,
    description = "`DecisionPayload` → `DecisionResult`. **Le service tente, il ne rejoue pas le graphe** : `programme.proposal_transitions_allowed` porte quatorze lignes, et le déclencheur en est l'arbitre — il refuse ce qui n'est pas déclaré, exige le motif quand la règle le dit, date la décision, journalise **et émet l'événement de domaine**. Le service n'émet donc rien : émettre à son tour produirait deux avis par décision, et le doublon ne se verrait qu'en production. **Les deux refus sortent en 200**, avec leur discriminant. `decision_reason` porte le motif de la dernière transition et rien de plus : le motif d'une décision se lit dans le journal.",
    path = "/proposals/{id}/decision",
    tag = "Back-office — propositions",
    operation_id = "propositions_decider",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "DecisionResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn decider(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<DecisionPayload>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    let ctx = contexte_de(&requete, perimetre.person_id);
    let corps = corps.into_inner();

    crate::service::perimeter::edition_dans_le_perimetre(
        state.pool(),
        &perimetre,
        crate::service::perimeter::Cible::Dossier(dossier),
    )
    .await?;

    let issue = transition::tenter(
        &state,
        &ctx,
        dossier,
        corps.to_status,
        corps.reason.as_deref(),
    )
    .await?;

    let resultat = match issue {
        transition::Issue::Appliquee(ligne) => {
            let fiche = crate::repo::proposals::fiche(state.pool(), dossier)
                .await?
                .ok_or_else(kernel::error::ApiError::not_found)?;
            DecisionResult::Applied {
                proposal: Box::new(fiche),
                transition: ligne,
            }
        }
        transition::Issue::TransitionInterdite(_) => DecisionResult::TransitionNotAllowed,
        transition::Issue::MotifExige => DecisionResult::ReasonRequired,
    };

    Ok(HttpResponse::Ok().json(resultat))
}
