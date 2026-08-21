//! Le dossier lu **par l'une ou l'autre voie**, et ce que chacune montre.
//!
//! # Deux voies d'accès, un seul refus
//!
//! « Accès au dossier » signifie : adhésion **active** à l'organisation
//! porteuse, **ou** lecture générale dans le périmètre de l'édition. Un membre
//! d'organisation n'a aucun périmètre ; un administrateur détaché n'est membre
//! d'aucune organisation. Dans tous les cas d'échec — inexistant, effacé, hors
//! périmètre, organisation étrangère — le refus est le **même**, et c'est ce
//! qui empêche une URL forgée de dire si le dossier existe.
//!
//! # 🔴 Ce que la voie de l'organisation ne doit PAS voir (écart n° 104)
//!
//! Le contrat du front décrit `Proposal` comme la ligne de table, agrégats
//! d'évaluation compris — note moyenne, note pondérée, élimination. Or FR-077
//! interdit qu'une note atteigne le déposant, et cette route lui est ouverte
//! par l'adhésion.
//!
//! **Le masquage est donc à la source** : ce qui n'est pas envoyé ne peut pas
//! fuiter, et un filtrage à l'affichage devrait être refait dans chaque écran,
//! chaque courriel et chaque export. Les deux notes partent nulles et
//! l'élimination part fausse — c'est-à-dire « rien à dire », l'état exact d'un
//! dossier que personne n'a encore noté.
//!
//! **`review_count` reste** : un nombre de revues déposées n'est ni une note,
//! ni un rang, et l'espace organisation affiche l'avancement de l'instruction.
//! Le rang, lui, n'est pas sur le dossier : il est calculé par la vue de
//! pilotage, que cette voie n'ouvre jamais.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::repo::cross;
use crate::repo::proposals::{self, Fiche};
use crate::service::perimeter::{self, Acces};
use crate::state::ProgrammeState;

/// Un dossier, **si ce lecteur y a accès**.
pub async fn dossier(state: &ProgrammeState, lecteur: Uuid, dossier: ProposalId) -> Result<Fiche> {
    let (_, acces) = perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;

    let fiche = proposals::fiche(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(selon_la_voie(fiche, acces))
}

/// Les dossiers d'une organisation, **par l'une ou l'autre voie**.
///
/// Membre actif : tous les dossiers de son organisation, toutes éditions
/// confondues — elle n'administre rien, aucun périmètre ne la borne.
/// Back-office : les mêmes, **bornés au périmètre d'administration**. Une
/// personne sans l'une ni l'autre reçoit le refus d'un dossier inexistant.
pub async fn de_lorganisation(
    state: &ProgrammeState,
    lecteur: Uuid,
    organisation: Uuid,
) -> Result<Vec<Fiche>> {
    let adhesion = cross::adhesion(state.pool(), organisation, lecteur).await?;
    if crate::domain::ownership::peut_agir(adhesion) {
        let fiches = proposals::de_lorganisation(state.pool(), organisation, None).await?;
        return Ok(fiches
            .into_iter()
            .map(|f| selon_la_voie(f, Acces::Organisation))
            .collect());
    }

    // La permission est exigée **quelque part**, le périmètre fait le tri
    // ensuite : un responsable détaché sur un webinaire lit les dossiers de
    // cette organisation sur son édition, et aucun autre.
    let perimetre = kernel::auth::administered_events(state.pool(), lecteur).await?;
    let autorise = kernel::auth::has_permission_anywhere(
        state.pool(),
        lecteur,
        crate::domain::permissions::PROPOSAL_READ_ALL,
    )
    .await?;

    if !autorise || perimetre.is_empty() {
        return Err(ApiError::not_found());
    }

    // Périmètre global : aucune borne d'édition. Périmètre listé : la liste.
    let editions = (!perimetre.is_global).then_some(perimetre.event_ids.as_slice());
    proposals::de_lorganisation(state.pool(), organisation, editions).await
}

/// **Ce que la voie de l'organisation ne voit pas.** Voir l'en-tête du fichier.
fn selon_la_voie(fiche: Fiche, acces: Acces) -> Fiche {
    match acces {
        Acces::BackOffice => fiche,
        Acces::Organisation => Fiche {
            average_score: None,
            weighted_score: None,
            is_knocked_out: false,
            ..fiche
        },
    }
}

// -----------------------------------------------------------------------------
// L'effacement logique
// -----------------------------------------------------------------------------

/// **Effacer un dossier — logiquement, avec son auteur et son motif, et en
/// purgeant ses thématiques** (écart n° 94).
///
/// # Pourquoi les thématiques se purgent à la main
///
/// `reference.entity_terms` est **polymorphe** : elle n'a aucune clé étrangère
/// vers `programme.proposals`, et aucune cascade ne l'atteint. La fonction de
/// purge annoncée par le modèle n'existe pas. Sans cet appel, les liens de
/// thématique d'un dossier effacé restent en base — invisibles, mais comptés
/// par tout ce qui agrège par thématique.
///
/// # Pourquoi l'effacement est logique
///
/// Un dossier porte un numéro communiqué à l'organisation, un journal, des
/// revues, des messages. Le détruire effacerait la trace d'une décision. La
/// vue de pilotage et toutes les lectures de ce module écartent déjà
/// `deleted_at`.
pub async fn effacer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
    motif: Option<&str>,
) -> Result<()> {
    perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        perimeter::Cible::Dossier(dossier),
    )
    .await?;

    let mut tx = state.db().write(ctx).await?;

    let efface = sqlx::query!(
        "UPDATE programme.proposals
            SET deleted_at = now(), deleted_by = $2, deleted_reason = $3
          WHERE id = $1 AND deleted_at IS NULL",
        dossier.as_uuid(),
        perimetre.person_id,
        motif.map(str::trim).filter(|m| !m.is_empty())
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if efface == 0 {
        tx.rollback().await?;
        return Err(ApiError::not_found());
    }

    crate::repo::themes::purger(&mut tx, dossier.as_uuid()).await?;
    tx.commit().await?;

    Ok(())
}
