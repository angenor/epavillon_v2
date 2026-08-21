//! Noter, déposer, se déporter.
//!
//! # 🔴 La consolidation est appelée ICI, ou nulle part
//!
//! `programme.refresh_proposal_score()` existe, son commentaire dit « à
//! appeler après toute saisie de note », et **aucun déclencheur ne l'appelle**.
//! Sans cet appel, la note du dossier, sa moyenne, son nombre de revues et son
//! élimination restent aux valeurs de la ligne : le classement du comité est
//! faux **sans qu'aucune erreur ne le signale** (écart n° 98).
//!
//! Elle est appelée **dans la transaction de l'écriture**, et les agrégats sont
//! **relus** ensuite : rendre une valeur calculée en Rust à côté d'une valeur
//! écrite en SQL produirait deux vérités pour le même dossier, et l'en-tête de
//! l'écran afficherait un classement que la liste contredirait.
//!
//! # Noter exige une AFFECTATION ; lire n'en exige pas
//!
//! Rien ne lie la permission à l'affectation en base (R21) : un membre du
//! comité détenant `programme.review.write` pourrait noter n'importe quel
//! dossier de son édition. Le service l'interdit — **et l'interdit aussi après
//! un déport**, sans quoi une déclaration d'impartialité se contredirait d'un
//! clic.
//!
//! **Mais lire reste permis** : un membre du comité ouvre un dossier qu'on ne
//! lui a pas confié sans le noter. Les deux règles sont décorrélées, et c'est
//! ce que `PROPOSAL_REVIEW_NOT_ASSIGNED` dit à l'écran — « masque la grille,
//! laisse la lecture ».
//!
//! # Ce service n'émet aucun événement
//!
//! La notation est **interne au comité** ; le déport aussi. Rien hors du module
//! n'en dépend, et l'avancement se relit à chaque ouverture de la fiche.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::domain::permissions::REVIEW_WRITE;
use crate::repo::assignments::Affectation;
use crate::repo::reviews::{Agregats, ChampsDeLaRevue, Revue};
use crate::repo::{assignments, cross, proposals, reviews, scores};
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// `SaveReviewPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SaveReviewPayload {
    pub recommendation: String,
    /// Indexé par critère. **Une entrée absente est une note non posée**, pas
    /// un zéro : zéro sur un critère éliminatoire disqualifie le dossier.
    #[serde(default)]
    pub scores: BTreeMap<Uuid, f64>,
    #[serde(default)]
    pub comments: BTreeMap<Uuid, String>,
    #[serde(default)]
    pub strengths: Option<String>,
    #[serde(default)]
    pub weaknesses: Option<String>,
    /// Visible du seul comité, **jamais du soumissionnaire**.
    #[serde(default)]
    pub private_note: Option<String>,
    /// `false` garde la revue en brouillon : elle ne compte dans aucun agrégat
    /// et reste invisible des pairs. `true` la dépose, déclenche le recalcul et
    /// **lève le voile**.
    pub submit: bool,
}

/// `SaveReviewResult` — la revue, et les agrégats **relus** du dossier.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ResultatDeNotation {
    pub review: Revue,
    pub proposal_weighted_score: Option<f64>,
    pub proposal_average_score: Option<f64>,
    pub review_count: i16,
    pub is_knocked_out: bool,
}

/// Les quatre recommandations, telles que la contrainte de la base les nomme.
const RECOMMANDATIONS: [&str; 4] = ["accept", "accept_with_changes", "neutral", "reject"];

/// Enregistrer ou déposer une revue.
pub async fn enregistrer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
    payload: SaveReviewPayload,
) -> Result<ResultatDeNotation> {
    let membre = perimetre.person_id;
    let edition = perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        perimeter::Cible::Dossier(dossier),
    )
    .await?;

    if !RECOMMANDATIONS.contains(&payload.recommendation.as_str()) {
        return Err(ApiError::validation(
            "Cette recommandation n'existe pas.",
            "recommendation",
        ));
    }

    exiger_de_pouvoir_noter(state, membre, edition.as_uuid(), dossier).await?;

    let grille = grille_du_dossier(state, dossier).await?;
    let notes = classer_les_notes(&payload, &grille)?;

    let mut tx = state.db().write(ctx).await?;

    let revue = reviews::enregistrer(
        &mut tx,
        dossier,
        membre,
        &ChampsDeLaRevue {
            recommendation: &payload.recommendation,
            strengths: payload.strengths.as_deref(),
            weaknesses: payload.weaknesses.as_deref(),
            private_note: payload.private_note.as_deref(),
            deposer: payload.submit,
        },
    )
    .await?;

    if let Err(erreur) = scores::remplacer(&mut tx, revue.id, &notes).await {
        tx.rollback().await?;
        return Err(nommer_le_critere(erreur, &grille));
    }

    // **La consolidation, dans la même transaction.** Hors d'elle, une fenêtre
    // existerait où la revue est écrite et le classement faux ; sans elle, le
    // classement resterait faux pour toujours.
    reviews::consolider(&mut tx, dossier).await?;

    // Les agrégats sont **relus**, jamais recalculés : l'autorité du calcul
    // reste en base.
    let revue = reviews::mienne(&mut *tx, dossier, membre)
        .await?
        .unwrap_or(revue);
    let agregats = reviews::agregats(&mut *tx, dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;

    tx.commit().await?;

    let Agregats {
        weighted_score,
        average_score,
        review_count,
        is_knocked_out,
    } = agregats;

    Ok(ResultatDeNotation {
        review: revue,
        proposal_weighted_score: weighted_score,
        proposal_average_score: average_score,
        review_count,
        is_knocked_out,
    })
}

/// `RecusalPayload` — **le motif est le sujet**.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecusalPayload {
    pub reason: String,
}

/// Se déporter d'un dossier.
///
/// **Le motif est obligatoire, et ce n'est pas une formalité** :
/// `review_assignments.recusal_reason` existe pour tracer l'impartialité du
/// comité. Un déport sans motif ne prouve rien et ne se relit pas six mois plus
/// tard, quand une organisation conteste.
///
/// **Le déport n'efface pas l'affectation** : il la date. La ligne demeure, et
/// c'est elle qui interdit une réattribution silencieuse.
pub async fn se_deporter(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
    payload: RecusalPayload,
) -> Result<Affectation> {
    let membre = perimetre.person_id;
    perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        perimeter::Cible::Dossier(dossier),
    )
    .await?;

    let motif = payload.reason.trim();
    if motif.is_empty() {
        return Err(ApiError::validation(
            "Le motif du déport est obligatoire : il trace l'impartialité du comité.",
            "reason",
        ));
    }

    let affectation = assignments::affectation(state.pool(), dossier, membre)
        .await?
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ProposalReviewNotAssigned,
                "Ce dossier ne vous a pas été confié : il n'y a rien dont vous déporter.",
            )
        })?;

    if affectation.recused_at.is_some() {
        return Ok(affectation);
    }

    let mut tx = state.db().write(ctx).await?;
    let ligne = sqlx::query!(
        "UPDATE programme.review_assignments
            SET recused_at = now(), recusal_reason = $3
          WHERE proposal_id = $1 AND reviewer_id = $2 AND recused_at IS NULL
      RETURNING id, proposal_id, reviewer_id, assigned_by, assigned_at,
                due_at, recused_at, recusal_reason",
        dossier.as_uuid(),
        membre,
        motif
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Affectation {
        id: ligne.id,
        proposal_id: ligne.proposal_id,
        reviewer_id: ligne.reviewer_id,
        assigned_by: ligne.assigned_by,
        assigned_at: ligne.assigned_at,
        due_at: ligne.due_at,
        recused_at: ligne.recused_at,
        recusal_reason: ligne.recusal_reason,
    })
}

// -----------------------------------------------------------------------------
// Les gardes
// -----------------------------------------------------------------------------

/// **Permission ET affectation non déportée.** Les deux, et dans cet ordre.
async fn exiger_de_pouvoir_noter(
    state: &ProgrammeState,
    membre: Uuid,
    edition: Uuid,
    dossier: ProposalId,
) -> Result<()> {
    let autorise = kernel::auth::has_permission(
        state.pool(),
        membre,
        REVIEW_WRITE,
        kernel::auth::Scope::Event(edition),
    )
    .await?;

    if !autorise {
        return Err(ApiError::with_message(
            ErrorCode::ProposalReviewNotAssigned,
            "Vous n'évaluez pas les dossiers de cette édition.",
        ));
    }

    match assignments::affectation(state.pool(), dossier, membre).await? {
        Some(a) if a.recused_at.is_none() => Ok(()),
        Some(_) => Err(ApiError::with_message(
            ErrorCode::ProposalReviewNotAssigned,
            "Vous vous êtes déporté de ce dossier : vous ne pouvez plus le noter.",
        )),
        None => Err(ApiError::with_message(
            ErrorCode::ProposalReviewNotAssigned,
            "Ce dossier ne vous a pas été confié.",
        )),
    }
}

/// La grille de l'appel du dossier — vide pour un dossier hors appel.
async fn grille_du_dossier(
    state: &ProgrammeState,
    dossier: ProposalId,
) -> Result<Vec<cross::Critere>> {
    let etat = proposals::etat(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;

    match etat.call_id {
        Some(call_id) => cross::grille_de_lappel(state.pool(), call_id).await,
        None => Ok(Vec::new()),
    }
}

/// Rapprocher les notes reçues de la grille — **et refuser un critère
/// étranger**.
///
/// Écrire une note sur un critère d'un autre appel se heurterait à la clé
/// étrangère et rendrait `PROPOSAL_UNKNOWN_REFERENCE` sans dire lequel. Le
/// classement préalable le nomme, comme le refus de plafond nomme le sien.
fn classer_les_notes(
    payload: &SaveReviewPayload,
    grille: &[cross::Critere],
) -> Result<Vec<scores::NoteAPoser>> {
    let mut notes = Vec::with_capacity(payload.scores.len());

    for (critere, valeur) in &payload.scores {
        let connu = grille.iter().find(|c| &c.id == critere).ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ProposalUnknownReference,
                format!("Le critère {critere} n'appartient pas à la grille de cet appel."),
            )
            .field("scores")
        })?;

        if *valeur < 0.0 {
            return Err(ApiError::validation(
                format!(
                    "La note du critère « {} » ne peut pas être négative.",
                    connu.code
                ),
                "scores",
            ));
        }

        notes.push(scores::NoteAPoser {
            criterion_id: *critere,
            score: *valeur,
            comment: payload
                .comments
                .get(critere)
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .map(str::to_owned),
        });
    }

    Ok(notes)
}

/// Traduire le refus de plafond **en nommant le critère et sa borne**.
///
/// Le déclencheur lève `check_violation` avec un message français qui
/// interpole la note et le maximum, mais pas le critère : il ne connaît que
/// son identifiant. Le service, lui, a la grille en main.
fn nommer_le_critere(erreur: ApiError, grille: &[cross::Critere]) -> ApiError {
    if erreur.code != ErrorCode::ValidationFailed {
        return erreur;
    }

    let bornes = grille
        .iter()
        .map(|c| format!("« {} » sur {}", c.code, c.max_score))
        .collect::<Vec<_>>()
        .join(", ");

    ApiError::with_message(
        ErrorCode::ValidationFailed,
        format!("{} Bornes de cette grille : {bornes}.", erreur.message),
    )
    .field("scores")
}
