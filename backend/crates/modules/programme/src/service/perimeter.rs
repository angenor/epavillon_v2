//! Remonter du message, du dossier ou de la revue **jusqu'à l'édition**, puis
//! vérifier le périmètre (R13).
//!
//! # L'ordre est imposé et ne se négocie pas
//!
//! **Résoudre l'ascendance, PUIS vérifier le périmètre, PUIS agir.** Vérifier
//! d'abord reviendrait à croire l'édition que le client annonce — et le contrat
//! du front envoie encore `event_id` dans ses charges utiles, un droit *déclaré
//! par le client*. Il est ignoré, comme `actorId` et `organization_id` l'ont été
//! en B1, B2 et B3.
//!
//! # La nuance de ce module : trois niveaux
//!
//! B3 remontait d'un enfant à son édition. Ici, un **message** appartient à un
//! **dossier**, qui appartient à une **édition** : la remontée est de deux
//! sauts, et la revue suit le même chemin.
//!
//! # Ce que la résolution ne divulgue jamais
//!
//! Elle **ne rend rien à l'appelant avant que le contrôle ne soit passé**, et
//! l'absence de l'objet produit le **même** refus que l'échec du périmètre : un
//! identifiant inexistant et un identifiant hors périmètre sont indiscernables
//! par la forme de la réponse (principe IX). Sans cela, une URL forgée dirait à
//! qui la forge si le dossier existe.
//!
//! # Ce qui ne passe pas par ici
//!
//! **L'espace organisation.** Il est borné par l'**adhésion active**, pas par
//! le périmètre d'administration : une organisation n'administre rien. Les deux
//! voies d'accès à un dossier sont distinctes et testées séparément —
//! `domain/ownership.rs` porte l'autre.

use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use sqlx::PgPool;

use crate::domain::ids::{CommentId, EventId, ProposalId, ReviewId};
use crate::repo::cross;

/// Ce qu'une route paramétrée désigne. Quatre portes d'entrée, et c'est la
/// liste exhaustive.
#[derive(Debug, Clone, Copy)]
pub enum Cible {
    Edition(EventId),
    Dossier(ProposalId),
    Message(CommentId),
    Revue(ReviewId),
}

/// **Résoudre l'ascendance, puis vérifier le périmètre.**
pub async fn edition_dans_le_perimetre(
    pool: &PgPool,
    perimetre: &Perimeter,
    cible: Cible,
) -> Result<EventId> {
    let edition = match cible {
        Cible::Edition(id) => cross::event_exists(pool, id).await?.then_some(id),
        Cible::Dossier(id) => cross::event_id_of_proposal(pool, id).await?,
        Cible::Message(id) => cross::event_id_of_comment(pool, id).await?,
        Cible::Revue(id) => cross::event_id_of_review(pool, id).await?,
    };

    let edition = edition.ok_or_else(ApiError::not_found)?;
    perimetre.ensure(edition.as_uuid())?;
    Ok(edition)
}

/// L'édition d'un dossier **sans contrôle de périmètre** — la voie de
/// l'organisation.
///
/// Elle existe parce que le déposant a besoin de connaître le fuseau et
/// l'échéance de l'édition de son dossier, et qu'il n'en administre aucune. Le
/// contrôle qui la remplace est l'adhésion active, vérifié par l'appelant
/// **avant** de s'en servir : cette fonction ne garde rien, et son nom le dit.
pub async fn edition_du_dossier_sans_garde(pool: &PgPool, dossier: ProposalId) -> Result<EventId> {
    cross::event_id_of_proposal(pool, dossier)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// **Une action groupée s'évalue dossier par dossier.**
///
/// Une sélection de douze peut traverser deux éditions, et le périmètre
/// s'applique à chacune. Rendre `None` pour un dossier hors périmètre ou
/// inexistant permet à l'appelant de le porter en **écart** de l'action
/// groupée (`BulkSkip.not_found`) au lieu d'échouer l'ensemble — ce qui est
/// précisément la différence entre un dossier introuvable seul (404) et un
/// dossier introuvable parmi douze (200 avec son écart).
pub async fn edition_si_dans_le_perimetre(
    pool: &PgPool,
    perimetre: &Perimeter,
    dossier: ProposalId,
) -> Result<Option<EventId>> {
    let edition = cross::event_id_of_proposal(pool, dossier).await?;
    Ok(edition.filter(|e| perimetre.allows(e.as_uuid())))
}

// -----------------------------------------------------------------------------
// « Accès au dossier » — DEUX voies, distinctes et testées séparément
// -----------------------------------------------------------------------------

/// Par où l'accès a été accordé. La distinction n'est pas cosmétique : ce que
/// l'espace organisation rend n'est pas ce que la fiche du comité rend, et
/// l'appelant doit savoir de quel côté il se trouve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acces {
    /// Adhésion **active** à l'organisation porteuse. Une organisation
    /// n'administre rien : ce n'est pas un périmètre.
    Organisation,
    /// Lecture générale **dans le périmètre** de l'édition du dossier.
    BackOffice,
}

/// Résoudre l'accès à un dossier, **par l'une ou l'autre voie**.
///
/// « Accès au dossier » signifie : adhésion active à l'organisation porteuse,
/// **ou** lecture générale dans le périmètre de l'édition. Les deux voies sont
/// distinctes et testées séparément — un membre d'organisation n'a aucun
/// périmètre, et un administrateur détaché n'est membre d'aucune organisation.
///
/// **L'ordre compte peu, le refus compte beaucoup** : dans tous les cas d'échec
/// — dossier inexistant, effacé, hors périmètre, organisation étrangère — le
/// refus est le **même**, et c'est ce qui empêche une URL forgée de dire si le
/// dossier existe.
pub async fn acces_au_dossier(
    pool: &PgPool,
    personne: uuid::Uuid,
    dossier: ProposalId,
) -> Result<(EventId, Acces)> {
    let ligne = sqlx::query!(
        "SELECT event_id, organization_id FROM programme.proposals
          WHERE id = $1 AND deleted_at IS NULL",
        dossier.as_uuid()
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(ApiError::not_found)?;

    let edition = EventId::from(ligne.event_id);

    let adhesion = cross::adhesion(pool, ligne.organization_id, personne).await?;
    if crate::domain::ownership::peut_agir(adhesion) {
        return Ok((edition, Acces::Organisation));
    }

    let lecture = kernel::auth::has_permission(
        pool,
        personne,
        crate::domain::permissions::PROPOSAL_READ_ALL,
        kernel::auth::Scope::Event(ligne.event_id),
    )
    .await?;
    let perimetre = kernel::auth::administered_events(pool, personne).await?;

    if lecture && perimetre.allows(ligne.event_id) {
        Ok((edition, Acces::BackOffice))
    } else {
        Err(ApiError::not_found())
    }
}
