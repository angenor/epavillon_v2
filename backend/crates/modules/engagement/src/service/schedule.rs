//! **Le calendrier des rappels d'une séance — quatre lignes, et pas un nom.**
//!
//! C'est l'écart n° 34 : depuis la v1, une organisation qui anime une séance ne
//! sait pas ce qui part à ses inscrits.
//!
//! # Un nombre, jamais une liste
//!
//! La garantie n'est pas tenue par la discipline de ce fichier mais par la
//! **signature** de `engagement.session_reminder_schedule()`, qui ne rend ni
//! identifiant de personne, ni nom, ni adresse. Un test balaie la charge utile
//! **sérialisée entière** plutôt que champ par champ : champ par champ
//! laisserait passer celui qu'on ajoutera demain (FR-048).
//!
//! # `has_rule` n'est pas décoratif
//!
//! Une liste vide se confond avec « tout est parti ». Les deux situations
//! demandent des mots différents à l'écran, et lui laisser deviner serait lui
//! demander d'inventer (FR-051).
//!
//! # La garde n'est PAS un périmètre d'administration
//!
//! **Une organisation n'administre rien.** Son accès passe par l'adhésion
//! **active** à l'organisation qui anime — celle du dossier dont la séance est
//! issue —, et l'autre porte est `programme.registration.manage` **sur
//! l'édition de cette séance**, déjà bornée par sa portée (FR-054).

use kernel::auth::{has_permission, Scope};
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::reminder::{ApplicableReminderRule, SessionReminderSchedule};
use crate::repo::{cross, reminders, rules};
use crate::state::EngagementState;

/// Le droit de gérer les inscriptions d'une édition. Testé **par permission**,
/// jamais par nom de rôle, et sur la portée de l'édition de la séance visée.
const PERMISSION: &str = "programme.registration.manage";

/// Le calendrier d'une séance, gardé.
pub async fn calendrier(
    state: &EngagementState,
    acteur: Uuid,
    session_id: Uuid,
) -> Result<SessionReminderSchedule> {
    let seance = exiger_le_droit_sur_la_seance(state, acteur, session_id).await?;

    let applicable = rules::applicable(state.pool(), session_id, seance.event_id).await?;
    let slots = reminders::calendrier(state.pool(), session_id).await?;

    Ok(SessionReminderSchedule {
        slots,
        has_rule: applicable.is_some(),
    })
}

/// La règle **applicable** à une séance, avec son origine — gardée par la même
/// porte que le calendrier, puisqu'elle répond à la même question sur la même
/// séance.
pub async fn regle_applicable(
    state: &EngagementState,
    acteur: Uuid,
    session_id: Uuid,
) -> Result<Option<ApplicableReminderRule>> {
    let seance = exiger_le_droit_sur_la_seance(state, acteur, session_id).await?;
    rules::applicable(state.pool(), session_id, seance.event_id).await
}

/// **Adhésion active, ou droit de gérer les inscriptions de l'édition.**
///
/// Le refus est un 403 et non un 404 : une séance figure au programme public,
/// et lui refuser son existence n'apprendrait rien à personne — alors que dire
/// « vous n'y avez pas droit » à une membre dont l'adhésion vient d'être
/// suspendue lui dit quoi faire. Une séance **inconnue**, elle, reste un 404.
async fn exiger_le_droit_sur_la_seance(
    state: &EngagementState,
    acteur: Uuid,
    session_id: Uuid,
) -> Result<cross::SeanceVisee> {
    let Some(seance) = cross::seance(state.pool(), session_id).await? else {
        return Err(ApiError::not_found());
    };

    if let Some(organisation) = seance.organization_id {
        if cross::adhesion_active(state.pool(), acteur, organisation).await? {
            return Ok(seance);
        }
    }

    if has_permission(
        state.pool(),
        acteur,
        PERMISSION,
        Scope::Event(seance.event_id),
    )
    .await?
    {
        return Ok(seance);
    }

    Err(ApiError::forbidden())
}

// -----------------------------------------------------------------------------
// Ce que le consommateur d'outbox déclenche
// -----------------------------------------------------------------------------
//
// Ces gestes vivent sur une **connexion**, pas sur l'état du module : ils sont
// appelés dans la transaction du relais d'outbox, et l'effet doit être défait
// avec l'événement si celui-ci échoue.

use crate::domain::reminder::motifs;
use sqlx::postgres::PgConnection;

/// **Matérialise les rappels d'une séance**, en réactivant d'abord ce qui
/// pouvait l'être.
///
/// L'ordre est le cœur de R21. `ux_scheduled_reminders_once` porte sur (séance,
/// personne, canal, décalage) **sans condition d'état** : une ligne annulée
/// existe toujours, et la fonction du modèle — qui insère en `ON CONFLICT DO
/// NOTHING` — ne la ressuscite pas. Matérialiser sans réactiver d'abord ne
/// produirait donc **rien** pour qui s'est désisté puis est revenu : aucune
/// erreur, aucune trace, et plus jamais de rappel.
///
/// **Rien n'est émis ni enfilé ici** : la fonction du modèle met un travail par
/// rappel en file et émet son annonce. Les redoubler produirait deux courriels
/// par rappel, et le doublon ne se verrait qu'en production.
pub async fn materialiser(
    conn: &mut PgConnection,
    session_id: Uuid,
    person_id: Option<Uuid>,
) -> Result<i32> {
    let reprises = reminders::reactiver(conn, session_id, person_id).await?;
    if reprises > 0 {
        tracing::info!(%session_id, reprises, "rappels réactivés : inscription reprise");
    }
    reminders::materialiser(conn, session_id).await
}

/// **Déplace les rappels d'un créneau qui bouge**, jamais ne les recrée.
///
/// Les recréer se heurterait à la clé d'unicité, qui ne porte pas l'instant :
/// les lignes resteraient à l'ancienne heure et rien ne le dirait. Le travail
/// déjà en file est déplacé **avec** la ligne : son échéance vit dans
/// `platform.jobs`, et une ligne remise à l'heure dont le travail ne bouge pas
/// enverrait le courriel à l'ancienne.
pub async fn deplacer(conn: &mut PgConnection, session_id: Uuid, secondes: f64) -> Result<u64> {
    let deplacement =
        reminders::decaler(conn, session_id, secondes, motifs::INSTANT_DEPASSE).await?;

    kernel::jobs::reschedule_by(conn, &deplacement.job_ids, secondes).await?;

    if deplacement.depasses > 0 {
        tracing::info!(
            %session_id,
            depasses = deplacement.depasses,
            "rappels écartés : le nouveau créneau les a laissés derrière"
        );
    }
    Ok(deplacement.deplaces)
}

/// Annule les rappels encore à traiter d'une séance — annulation ou report.
pub async fn annuler_la_seance(
    conn: &mut PgConnection,
    session_id: Uuid,
    motif: &str,
) -> Result<u64> {
    reminders::annuler(conn, session_id, None, motif).await
}

/// Annule les rappels d'une personne sur une séance — désistement, liste
/// d'attente, absence constatée.
pub async fn annuler_une_inscription(
    conn: &mut PgConnection,
    session_id: Uuid,
    person_id: Uuid,
) -> Result<u64> {
    reminders::annuler(
        conn,
        session_id,
        Some(person_id),
        motifs::INSCRIPTION_ANNULEE,
    )
    .await
}
