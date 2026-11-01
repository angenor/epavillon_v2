//! Les gestes de la régie du jour (02/11) et leur retour en arrière.
//!
//! Gardés comme le planificateur : séance dans le périmètre d'administration,
//! puis `programme.session.schedule` sur son édition. Chaque geste s'inscrit au
//! journal dans la même transaction que son effet ; le défaire relit l'état de
//! la séance et refuse si elle a changé entre-temps.

use kernel::auth::{Perimeter, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use serde_json::{json, Value};
use sqlx::postgres::PgConnection;
use time::Duration;
use uuid::Uuid;

use crate::domain::ids::{EventId, SessionId};
use crate::domain::permissions::SESSION_SCHEDULE;
use crate::domain::regie::{
    RegieActionKind, RegieCancelPayload, RegieGesture, RegieUndoPayload, DELAI_COURRIEL_MINUTES,
    RAISONS_ABSENCE, REPRISE_MINUTES,
};
use crate::repo::cross::regie as lecture;
use crate::repo::regie::{self as repo, Geste, SeanceVerrouillee};
use crate::service::perimeter::{self, Cible};
use crate::state::ProgrammeState;

/// Périmètre, puis permission de planifier sur l'édition résolue en base.
pub async fn garder(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    cible: Cible,
) -> Result<EventId> {
    let edition = perimeter::edition_dans_le_perimetre(state.pool(), perimetre, cible).await?;
    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        SESSION_SCHEDULE,
        Scope::Event(edition.as_uuid()),
    )
    .await?;
    Ok(edition)
}

fn etat_change() -> ApiError {
    ApiError::new(ErrorCode::RegieSessionStateChanged)
}

async fn seance(conn: &mut PgConnection, session_id: Uuid) -> Result<SeanceVerrouillee> {
    repo::verrouiller_seance(conn, session_id)
        .await?
        .ok_or_else(ApiError::not_found)
}

fn geste(g: &Geste, registrants: i64) -> RegieGesture {
    RegieGesture {
        action_id: g.id,
        kind: RegieActionKind::parse(&g.action_code).unwrap_or(RegieActionKind::LiveStarted),
        session_id: g.session_id,
        acted_at: g.acted_at,
        notify_at: None,
        registrants,
        resumable_until: None,
        undone_at: g.undone_at,
    }
}

/// Règle n° 4, toutes éditions confondues. Le titre n'est dit que pour une
/// séance de la même édition : celle d'une autre COP peut être hors périmètre.
async fn un_seul_direct(conn: &mut PgConnection, session: &SeanceVerrouillee) -> Result<()> {
    repo::verrouiller_le_direct(conn).await?;
    let Some(autre) = lecture::direct_en_cours(&mut *conn, Some(session.id)).await? else {
        return Ok(());
    };
    let titre = autre.title.get("fr").and_then(Value::as_str);
    Err(match titre.filter(|_| autre.event_id == session.event_id) {
        Some(t) => ApiError::with_message(
            ErrorCode::RegieLiveAlreadyRunning,
            format!("« {t} » est en direct : un seul direct à la fois. Terminez-le d'abord."),
        ),
        None => ApiError::new(ErrorCode::RegieLiveAlreadyRunning),
    })
}

pub async fn confirmer_presence(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<RegieGesture> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let mut tx = state.db().write(ctx).await?;
    let s = seance(&mut tx, session_id).await?;
    if s.status == "cancelled" {
        return Err(etat_change());
    }
    let avant = repo::presence(&mut tx, session_id).await?;
    repo::poser_presence(&mut tx, session_id, Some(true)).await?;
    let g = repo::inscrire_geste(
        &mut tx,
        s.event_id,
        session_id,
        RegieActionKind::PresenceConfirmed.as_str(),
        json!({ "previous": avant }),
    )
    .await?;
    tx.commit().await?;
    Ok(geste(&g, 0))
}

pub async fn annuler(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
    charge: RegieCancelPayload,
) -> Result<RegieGesture> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let note = charge
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if note.is_some_and(|n| n.chars().count() > 2000) {
        return Err(ApiError::validation(
            "La note tient en deux mille caractères.",
            "note",
        ));
    }

    let mut tx = state.db().write(ctx).await?;
    if !repo::raison_connue(&mut tx, &charge.reason_code).await? {
        return Err(ApiError::new(ErrorCode::RegieCancellationReasonUnknown).field("reason_code"));
    }
    let s = seance(&mut tx, session_id).await?;
    if !s.is_published || !matches!(s.status.as_str(), "scheduled" | "live" | "postponed") {
        return Err(etat_change());
    }
    let presence_avant = repo::presence(&mut tx, session_id).await?;
    if RAISONS_ABSENCE.contains(&charge.reason_code.as_str()) {
        repo::poser_presence(&mut tx, session_id, Some(false)).await?;
    }
    repo::annuler(&mut tx, session_id, &charge.reason_code, note).await?;

    let inscrits = repo::inscrits(&mut tx, session_id).await?;
    let prevenir = charge.notify_registrants && inscrits > 0;
    let echeance = s.now + Duration::minutes(DELAI_COURRIEL_MINUTES);
    let g = repo::inscrire_geste(
        &mut tx,
        s.event_id,
        session_id,
        RegieActionKind::SessionCancelled.as_str(),
        json!({
            "previous_status": s.status,
            "previous_presence": presence_avant,
            "reason_code": charge.reason_code,
            "notify_registrants": prevenir,
            "notify_at": prevenir.then(|| echeance.format(&time::format_description::well_known::Rfc3339).ok()).flatten(),
            "registrants": inscrits,
        }),
    )
    .await?;
    if prevenir {
        avis(
            &mut tx,
            contracts::programme::REGIE_CANCELLATION_NOTICE,
            g.id,
            session_id,
            echeance,
        )
        .await?;
    }
    tx.commit().await?;

    let mut rendu = geste(&g, inscrits);
    rendu.notify_at = prevenir.then_some(echeance);
    Ok(rendu)
}

async fn avis(
    conn: &mut PgConnection,
    type_: &str,
    action_id: Uuid,
    session_id: Uuid,
    notify_at: time::OffsetDateTime,
) -> Result<()> {
    let charge = serde_json::to_value(contracts::programme::RegieNotice {
        action_id,
        session_id,
        notify_at,
    })
    .map_err(ApiError::internal)?;
    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: "programme",
            aggregate_type: "session",
            aggregate_id: session_id,
            event_type: type_,
            payload: charge,
        },
    )
    .await?;
    Ok(())
}

pub async fn demarrer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<RegieGesture> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let mut tx = state.db().write(ctx).await?;
    let s = seance(&mut tx, session_id).await?;
    if !s.is_published || s.status != "scheduled" {
        return Err(etat_change());
    }
    un_seul_direct(&mut tx, &s).await?;
    repo::changer_statut(&mut tx, session_id, "live").await?;
    let g = repo::inscrire_geste(
        &mut tx,
        s.event_id,
        session_id,
        RegieActionKind::LiveStarted.as_str(),
        json!({ "previous_status": s.status }),
    )
    .await?;
    tx.commit().await?;
    Ok(geste(&g, 0))
}

pub async fn terminer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<RegieGesture> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let mut tx = state.db().write(ctx).await?;
    let s = seance(&mut tx, session_id).await?;
    if s.status != "live" {
        return Err(etat_change());
    }
    repo::changer_statut(&mut tx, session_id, "completed").await?;
    let g = repo::inscrire_geste(
        &mut tx,
        s.event_id,
        session_id,
        RegieActionKind::LiveEnded.as_str(),
        json!({}),
    )
    .await?;
    tx.commit().await?;
    let mut rendu = geste(&g, 0);
    rendu.resumable_until = Some(g.acted_at + Duration::minutes(REPRISE_MINUTES));
    Ok(rendu)
}

/// Défaire une ligne du journal, même plus tard et par un autre membre de
/// l'équipe. Un courriel parti ne se reprend pas : rétablir une annulation
/// envoie le correctif à ceux qui l'ont reçu, si on le demande.
pub async fn defaire(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    action_id: Uuid,
    charge: RegieUndoPayload,
) -> Result<RegieGesture> {
    let session_id = {
        let mut conn = state.pool().acquire().await?;
        repo::session_du_geste(&mut conn, action_id)
            .await?
            .ok_or_else(ApiError::not_found)?
    };
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;

    let mut tx = state.db().write(ctx).await?;
    let g = repo::verrouiller_geste(&mut tx, action_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if g.undone_at.is_some() {
        return Err(ApiError::new(ErrorCode::RegieActionAlreadyUndone));
    }
    let s = seance(&mut tx, g.session_id).await?;
    let kind = RegieActionKind::parse(&g.action_code)
        .ok_or_else(|| ApiError::internal(format!("geste de régie inconnu : {}", g.action_code)))?;
    let mut ajout = json!({});
    let mut inscrits = 0;

    match kind {
        RegieActionKind::PresenceConfirmed => {
            let avant = g.data.get("previous").and_then(Value::as_bool);
            repo::poser_presence(&mut tx, g.session_id, avant).await?;
        }
        RegieActionKind::SessionCancelled => {
            if s.status != "cancelled" {
                return Err(etat_change());
            }
            // Un direct annulé revient programmé : le relancer reste un geste.
            let avant = g
                .data
                .get("previous_status")
                .and_then(Value::as_str)
                .filter(|st| *st != "live")
                .unwrap_or("scheduled");
            repo::changer_statut(&mut tx, g.session_id, avant).await?;
            let presence = g.data.get("previous_presence").and_then(Value::as_bool);
            repo::poser_presence(&mut tx, g.session_id, presence).await?;

            let echeance = g
                .data
                .get("notify_at")
                .and_then(Value::as_str)
                .and_then(|t| {
                    time::OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339)
                        .ok()
                });
            let deja_du = echeance.is_some_and(|e| e <= s.now);
            let correctif = charge.notify_registrants && deja_du;
            ajout = json!({ "restore_notice": correctif });
            inscrits = repo::inscrits(&mut tx, g.session_id).await?;
            if correctif {
                avis(
                    &mut tx,
                    contracts::programme::REGIE_RESTORATION_NOTICE,
                    g.id,
                    g.session_id,
                    s.now,
                )
                .await?;
            }
        }
        RegieActionKind::LiveStarted => {
            if s.status != "live" {
                return Err(etat_change());
            }
            let avant = g
                .data
                .get("previous_status")
                .and_then(Value::as_str)
                .unwrap_or("scheduled");
            repo::changer_statut(&mut tx, g.session_id, avant).await?;
        }
        RegieActionKind::LiveEnded => {
            if s.status != "completed" {
                return Err(etat_change());
            }
            let fin = s.live_ended_at.unwrap_or(g.acted_at);
            if s.now > fin + Duration::minutes(REPRISE_MINUTES) {
                return Err(ApiError::new(ErrorCode::RegieResumeExpired));
            }
            un_seul_direct(&mut tx, &s).await?;
            repo::changer_statut(&mut tx, g.session_id, "live").await?;
        }
    }

    let defait = repo::marquer_defait(&mut tx, g.id, ajout).await?;
    tx.commit().await?;
    let mut rendu = geste(&g, inscrits);
    rendu.undone_at = Some(defait);
    Ok(rendu)
}
