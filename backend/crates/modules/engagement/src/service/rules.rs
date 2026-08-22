//! **L'administrateur décide ce qui part, et voit ce qui va partir.**
//!
//! # Cette histoire passe avant celles qui la précèdent en priorité
//!
//! **Rien ne sème de règle de rappel** : sur une base neuve, aucune édition n'en
//! a (écart n° 130). Sans écriture de règle, ni le calendrier ni les envois ne
//! se démontrent autrement qu'en posant une ligne à la main en SQL — ce qui
//! prouverait la lecture sans prouver le chemin réel. L'écriture est donc
//! **l'instrument de mesure des deux autres histoires**.
//!
//! # Une LISTE de décalages, jamais un décalage seul
//!
//! Les quatre décalages du défaut sont **cumulés** : ce n'est pas un choix parmi
//! quatre, les quatre rappels partent. Une écriture qui n'accepterait qu'une
//! valeur ferait croire le contraire à l'écran, et la faute ne se verrait qu'au
//! jour de la séance (FR-070).
//!
//! # La règle de séance REMPLACE celle de l'édition
//!
//! Sans cumul, et c'est ce qui permet à l'administrateur de savoir ce qui va
//! partir. La lecture rend donc l'**origine** de la règle applicable : sans
//! elle, une règle de séance à deux décalages ne se distingue pas d'une règle
//! d'édition qu'on aurait tronquée (FR-074, FR-075).
//!
//! # Le droit se vérifie sur la portée VISÉE
//!
//! Une règle de séance se garde sur l'édition **de cette séance**, pas sur celle
//! que l'appelant administre par ailleurs. Règle métier n° 8, y compris quand
//! l'utilisateur forge une URL : la permission **et** le périmètre.

use kernel::auth::{administered_events, has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::RequestContext;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::offsets;
use crate::domain::reminder::{motifs, ApplicableReminderRule, NotificationChannel, ReminderRule};
use crate::repo::{cross, rules};
use crate::state::EngagementState;

/// La permission qui gouverne les trois routes. Testée **par permission**,
/// jamais par nom de rôle, et toujours avec sa portée.
const PERMISSION: &str = "engagement.reminder.manage";

/// Ce qu'une écriture de règle déclare — `ReminderRulePayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReminderRulePayload {
    /// **Exactement l'un des deux.** Le modèle l'exige
    /// (`ck_reminder_rules_scope`), et le refus sort sur le champ `scope`.
    pub event_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    /// En minutes, **cumulés**. Absent : le défaut du modèle — 2 jours, 1 jour,
    /// 1 heure, 30 minutes.
    pub offsets: Option<Vec<i32>>,
    /// Absent : `email`, le seul canal que ce jalon sait servir.
    pub channels: Option<Vec<String>>,
    pub type_code: Option<String>,
    pub template_id: Option<Uuid>,
    /// Absent : active. **Couper sans supprimer** se fait ici ; supprimer
    /// annule en plus les rappels encore à traiter.
    pub is_active: Option<bool>,
}

/// Le type de notification par défaut, celui que le modèle pose lui-même sur la
/// colonne.
const TYPE_PAR_DEFAUT: &str = "programme.session.reminder";

/// La portée d'une règle, résolue et gardée.
struct Portee {
    event_id: Uuid,
    session_id: Option<Uuid>,
}

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

/// Les règles d'une édition : la sienne, et celles de ses séances.
pub async fn lister(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<Vec<ReminderRule>> {
    exiger_le_droit_sur_ledition(state, acteur, event_id).await?;
    let seances = cross::seances_de_ledition(state.pool(), event_id).await?;
    rules::par_edition(state.pool(), event_id, &seances).await
}

/// **La règle applicable à une séance**, avec son origine. `None` : aucune ne
/// s'applique, et l'écran doit le dire plutôt que d'afficher une liste vide
/// (FR-076).
///
/// **Cette lecture n'est pas gardée ici** : sa garde est celle de sa route —
/// adhésion active à l'organisation qui anime, ou permission sur l'édition —, et
/// cette route arrive avec US4. La poser ici la dédoublerait avec celle du
/// calendrier, qui répond à la même question sur la même séance.
pub async fn applicable(
    state: &EngagementState,
    session_id: Uuid,
) -> Result<Option<ApplicableReminderRule>> {
    let Some(seance) = cross::seance(state.pool(), session_id).await? else {
        return Err(ApiError::not_found());
    };
    rules::applicable(state.pool(), session_id, seance.event_id).await
}

// -----------------------------------------------------------------------------
// Écriture
// -----------------------------------------------------------------------------

/// Écrit une règle, ou **modifie** celle qui existait pour la même portée.
pub async fn ecrire(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &ReminderRulePayload,
) -> Result<ReminderRule> {
    let portee = resoudre_la_portee(state, acteur, payload).await?;

    let minutes = payload
        .offsets
        .clone()
        .unwrap_or_else(|| offsets::DEFAUT_MINUTES.to_vec());
    if !offsets::sont_valides(&minutes) {
        return Err(refus_de_decalages(&minutes));
    }

    let canaux = payload
        .channels
        .clone()
        .unwrap_or_else(|| vec![NotificationChannel::Email.as_str().to_owned()]);
    verifier_les_canaux(&canaux)?;

    let valeurs = rules::ValeursDeRegle {
        offsets: offsets::ranges(&minutes),
        channels: canaux,
        type_code: payload
            .type_code
            .clone()
            .unwrap_or_else(|| TYPE_PAR_DEFAUT.to_owned()),
        template_id: payload.template_id,
        is_active: payload.is_active.unwrap_or(true),
        created_by: acteur,
    };

    let mut tx = state.db().write(ctx).await?;
    let ecriture = match portee.session_id {
        Some(session_id) => rules::ecrire_pour_seance(&mut tx, session_id, &valeurs).await,
        None => rules::ecrire_pour_edition(&mut tx, portee.event_id, &valeurs).await,
    };

    let rule_id = match ecriture {
        Ok(id) => id,
        Err(erreur) => {
            tx.rollback().await?;
            return Err(traduire(erreur));
        }
    };
    tx.commit().await?;

    rules::par_id(state.pool(), rule_id)
        .await?
        .ok_or_else(|| ApiError::internal("règle introuvable juste après son écriture"))
}

// -----------------------------------------------------------------------------
// Coupure
// -----------------------------------------------------------------------------

/// Supprime une règle **et annule les rappels encore à traiter qu'elle
/// gouvernait**, en rendant leur nombre (FR-078).
///
/// Les annuler est ce qui distingue une coupure d'un simple oubli : sans cela,
/// les rappels déjà matérialisés partiraient quand même, et l'administrateur
/// qui vient de retirer la règle les verrait arriver sans comprendre.
///
/// **L'ordre compte** : `scheduled_reminders.rule_id` est `ON DELETE SET NULL`,
/// et supprimer d'abord laisserait les rappels orphelins, donc introuvables.
pub async fn supprimer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    rule_id: Uuid,
) -> Result<i64> {
    let Some(regle) = rules::par_id(state.pool(), rule_id).await? else {
        return Err(ApiError::not_found());
    };
    exiger_le_droit_sur_la_regle(state, acteur, &regle).await?;

    let mut tx = state.db().write(ctx).await?;
    let annules = rules::annuler_les_rappels(&mut tx, rule_id, motifs::REGLE_RETIREE).await?;
    if !rules::supprimer(&mut tx, rule_id).await? {
        tx.rollback().await?;
        return Err(ApiError::not_found());
    }
    tx.commit().await?;

    Ok(annules)
}

// -----------------------------------------------------------------------------
// Les gardes, et les refus
// -----------------------------------------------------------------------------

/// La portée déclarée, vérifiée **et gardée**.
async fn resoudre_la_portee(
    state: &EngagementState,
    acteur: Uuid,
    payload: &ReminderRulePayload,
) -> Result<Portee> {
    match (payload.event_id, payload.session_id) {
        (Some(event_id), None) => {
            exiger_le_droit_sur_ledition(state, acteur, event_id).await?;
            Ok(Portee {
                event_id,
                session_id: None,
            })
        }
        (None, Some(session_id)) => {
            // **L'édition de CETTE séance**, et non celle que l'appelant
            // administre par ailleurs : sans quoi une URL forgée poserait une
            // règle sur la séance d'une COP voisine.
            let Some(seance) = cross::seance(state.pool(), session_id).await? else {
                return Err(ApiError::not_found());
            };
            exiger_le_droit_sur_ledition(state, acteur, seance.event_id).await?;
            Ok(Portee {
                event_id: seance.event_id,
                session_id: Some(session_id),
            })
        }
        // Les deux, ou aucun : le refus est celui de `ck_reminder_rules_scope`,
        // rendu avant l'écriture pour qu'il porte le champ `scope`.
        _ => Err(ApiError::new(ErrorCode::EngagementReminderScopeInvalid).field("scope")),
    }
}

/// La permission **sur la portée de l'édition**, et le périmètre
/// d'administration qui va avec.
async fn exiger_le_droit_sur_ledition(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<()> {
    if !cross::edition_existe(state.pool(), event_id).await? {
        return Err(ApiError::not_found());
    }

    let autorise = has_permission(state.pool(), acteur, PERMISSION, Scope::Event(event_id)).await?;
    let perimetre = administered_events(state.pool(), acteur).await?;

    // **Le refus prend la forme d'une absence** : un 403 dirait à qui forge une
    // URL que l'édition existe.
    if autorise && perimetre.allows(event_id) {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

/// La garde d'une règle existante : celle de l'édition qu'elle vise, ou celle de
/// l'édition de la séance qu'elle vise.
async fn exiger_le_droit_sur_la_regle(
    state: &EngagementState,
    acteur: Uuid,
    regle: &ReminderRule,
) -> Result<()> {
    let event_id = match (regle.event_id, regle.session_id) {
        (Some(event_id), _) => event_id,
        (None, Some(session_id)) => match cross::seance(state.pool(), session_id).await? {
            Some(seance) => seance.event_id,
            None => return Err(ApiError::not_found()),
        },
        _ => return Err(ApiError::not_found()),
    };

    exiger_le_droit_sur_ledition(state, acteur, event_id).await
}

/// Les canaux déclarés existent-ils ?
///
/// L'énuméré est fermé en base : un canal inconnu sortirait en erreur de cast,
/// sur un message qui ne dirait pas quel champ est en cause.
fn verifier_les_canaux(canaux: &[String]) -> Result<()> {
    if canaux.is_empty() {
        return Err(ApiError::new(ErrorCode::ValidationFailed)
            .field("channels")
            .detail("au moins un canal est exigé (ck_reminder_rules_channels)"));
    }
    for canal in canaux {
        if NotificationChannel::from_db(canal).is_none() {
            return Err(ApiError::new(ErrorCode::ValidationFailed)
                .field("channels")
                .detail(format!("canal inconnu : « {canal} »")));
        }
    }
    Ok(())
}

/// **Le refus des décalages dit lequel des trois cas s'applique.**
///
/// « Les délais doivent être compris entre un et huit valeurs, toutes
/// positives » ne dit pas laquelle est fautive. Le détail le dit, et le doublon
/// — que la base laisserait passer, `ux_scheduled_reminders_once` l'absorbant en
/// silence — est nommé lui aussi.
fn refus_de_decalages(minutes: &[i32]) -> ApiError {
    let motif = if minutes.len() < offsets::MIN_DECALAGES {
        "aucun décalage n'a été fourni".to_owned()
    } else if minutes.len() > offsets::MAX_DECALAGES {
        format!(
            "{} décalages fournis, {} au plus",
            minutes.len(),
            offsets::MAX_DECALAGES
        )
    } else if minutes.iter().any(|m| *m <= 0) {
        "un décalage doit être strictement positif : il se compte AVANT le début".to_owned()
    } else {
        "un décalage est répété ; la clé d'unicité du modèle l'absorberait en silence, \
         et l'écran annoncerait un envoi de plus qu'il n'y en aurait"
            .to_owned()
    };

    ApiError::new(ErrorCode::EngagementReminderOffsetsInvalid)
        .field("offsets")
        .detail(motif)
}

/// Ce que la base refuse malgré les contrôles amont.
///
/// Les deux contraintes portent leur **nom**, ce qui suffit à poser le refus sur
/// le bon champ — jamais le texte du message, qui se périme au premier
/// ajustement du SQL.
fn traduire(erreur: sqlx::Error) -> ApiError {
    match kernel::pg_error::constraint(&erreur) {
        Some("ck_reminder_rules_offsets") => {
            ApiError::new(ErrorCode::EngagementReminderOffsetsInvalid).field("offsets")
        }
        Some("ck_reminder_rules_scope") | Some("ck_reminder_rules_channels") => {
            ApiError::new(ErrorCode::EngagementReminderScopeInvalid).field("scope")
        }
        _ => ApiError::from(erreur),
    }
}
