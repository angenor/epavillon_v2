//! La file des doublons et ses décisions.
//!
//! **La file exige la permission de fusion en portée globale**, et ce n'est pas
//! un oubli : une paire ne relève d'aucune édition, et sa résolution exige de
//! toute façon la portée globale. Un administrateur détaché n'y accède **pas du
//! tout**, plutôt que d'en voir une part qui ne voudrait rien dire.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};

use crate::domain::duplicates::{
    DuplicateDecision, DuplicateDecisionOutcome, DuplicateQueueScreen,
};
use crate::domain::ids::{DuplicatePairId, PersonId};
use crate::repo::duplicates;
use crate::state::OrgState;

/// La file, dans ses deux sections.
pub async fn queue(state: &OrgState) -> Result<DuplicateQueueScreen> {
    Ok(DuplicateQueueScreen {
        pending: duplicates::en_attente(state.pool()).await?,
        settled: duplicates::tranchees(state.pool()).await?,
    })
}

/// Tranche une paire.
///
/// `distinct` la retire — le balayage ne la ressuscitera pas, c'est la clause
/// `WHERE reviewed_at IS NULL` de l'enregistrement qui le garantit. Mais rien
/// n'est définitif : `deferred` posé sur une paire déjà sortie de la file l'y
/// **ramène**, écartée comme reportée. Seule la fusion ne se reprend pas.
pub async fn decide(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    demande: DuplicateDecision,
) -> Result<DuplicateDecisionOutcome> {
    if !demande.est_recevable() {
        // `merged` ne se pose jamais depuis la file : c'est
        // `org.merge_organizations()` qui l'écrit, et elle seule.
        return Err(ApiError::validation(
            "Une décision de file vaut « distinct » ou « deferred » ; « merged » est posée par la fusion elle-même.",
            "decision",
        ));
    }

    let Some(pair_id) = demande.pair_id.map(DuplicatePairId) else {
        return Ok(DuplicateDecisionOutcome::NotFound);
    };

    let mut tx = state.db().write(ctx).await?;

    let Some(existante) = duplicates::par_identifiant(&mut tx, pair_id).await? else {
        return Ok(DuplicateDecisionOutcome::NotFound);
    };

    // Une paire fusionnée ne se rejuge pas : la réécrire effacerait la trace de
    // la fusion sans défaire la fusion elle-même.
    if existante.decision.as_deref() == Some("merged") {
        return Err(ApiError::validation(
            "Cette paire a été fusionnée ; sa décision ne se reprend pas.",
            "decision",
        ));
    }

    // Un report posé sur une paire DÉJÀ SORTIE de la file la ramène — quelle
    // qu'ait été la décision. C'est le geste « remettre dans la file », et il
    // vaut d'abord pour les paires écartées : ce sont celles qu'on se trompe en
    // écartant. Sur une paire en attente, le même report la met de côté.
    if demande.decision == "deferred" && existante.reviewed_at.is_some() {
        duplicates::remettre_en_circulation(&mut tx, pair_id).await?;
    } else {
        duplicates::trancher(&mut tx, pair_id, &demande.decision, acteur).await?;
    }

    let apres = duplicates::par_identifiant(&mut tx, pair_id).await?;
    tx.commit().await?;

    Ok(match apres {
        Some(pair) => DuplicateDecisionOutcome::Recorded {
            pair: Box::new(pair),
        },
        None => DuplicateDecisionOutcome::NotFound,
    })
}
