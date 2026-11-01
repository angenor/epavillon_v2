//! L'écran de la régie du jour (02/11) : une journée, son direct, son journal.
//! Lu dans une transaction en lecture seule : les séances, le journal et l'état
//! des courriels décrivent le même instant.

use std::collections::HashMap;

use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use serde_json::Value;
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::domain::regie::{
    JournalKind, NoticeState, PresenceState, RegieActionKind, RegieCancellation, RegieDocument,
    RegieJournalEntry, RegieLiveElsewhere, RegieMeeting, RegieNoticeStatus, RegieOrganization,
    RegiePresence, RegieReason, RegieScreen, RegieSession, REPRISE_MINUTES,
};
use crate::repo::cross::regie::{self as lecture, EnvoiDesCourriels, GesteDuJournal};
use crate::service::perimeter::Cible;
use crate::service::regie::garder;
use crate::state::ProgrammeState;

/// Aujourd'hui s'il est au programme, sinon le prochain jour, sinon le premier.
pub fn jour_par_defaut(jours: &[Date], aujourdhui: Date) -> Date {
    if jours.contains(&aujourdhui) {
        return aujourdhui;
    }
    jours
        .iter()
        .find(|j| **j > aujourdhui)
        .or_else(|| jours.first())
        .copied()
        .unwrap_or(aujourdhui)
}

fn echeance(g: &GesteDuJournal) -> Option<OffsetDateTime> {
    g.data
        .get("notify_at")
        .and_then(Value::as_str)
        .and_then(|t| OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339).ok())
}

/// Où en sont les courriels d'une annulation.
pub fn etat_des_courriels(
    g: &GesteDuJournal,
    envoi: Option<&EnvoiDesCourriels>,
) -> RegieNoticeStatus {
    let demande = g
        .data
        .get("notify_registrants")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let inscrits = g
        .data
        .get("registrants")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let du = echeance(g);
    let rien = |state| RegieNoticeStatus {
        state,
        due_at: None,
        sent_at: None,
        recipients: 0,
    };
    if !demande {
        return rien(NoticeState::None);
    }
    if let Some(e) = envoi.filter(|e| e.sent > 0) {
        return RegieNoticeStatus {
            state: NoticeState::Sent,
            due_at: du,
            sent_at: e.last_sent_at.or(e.done_at),
            recipients: e.sent,
        };
    }
    if g.undone_at.is_some() || envoi.is_some_and(|e| e.done_at.is_some()) {
        return rien(NoticeState::Withdrawn);
    }
    RegieNoticeStatus {
        state: NoticeState::Pending,
        due_at: du,
        sent_at: None,
        recipients: inscrits,
    }
}

/// Le geste se défait-il encore ? L'état de la séance doit être celui que le
/// geste a laissé ; un direct terminé ne se reprend que dans les quinze minutes.
fn defaisable(g: &GesteDuJournal, maintenant: OffsetDateTime) -> bool {
    if g.undone_at.is_some() {
        return false;
    }
    match RegieActionKind::parse(&g.action_code) {
        Some(RegieActionKind::PresenceConfirmed) => g.session_status != "cancelled",
        Some(RegieActionKind::SessionCancelled) => g.session_status == "cancelled",
        Some(RegieActionKind::LiveStarted) => g.session_status == "live",
        Some(RegieActionKind::LiveEnded) => {
            g.session_status == "completed"
                && g.session_live_ended_at.unwrap_or(g.acted_at)
                    + Duration::minutes(REPRISE_MINUTES)
                    >= maintenant
        }
        None => false,
    }
}

pub async fn ecran(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    event_id: Uuid,
    jour_demande: Option<Date>,
) -> Result<RegieScreen> {
    garder(state, perimetre, Cible::Edition(EventId(event_id))).await?;

    let mut tx = state.pool().begin().await?;
    sqlx::query!("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    let edition = lecture::edition_de_la_regie(&mut *tx, event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let maintenant = edition.now;
    let jours = lecture::jours_du_programme(&mut *tx, event_id).await?;
    let jour = jour_demande.unwrap_or_else(|| jour_par_defaut(&jours, edition.today));

    let lignes = lecture::seances_du_jour(&mut *tx, event_id, jour).await?;
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    let mut organisations: HashMap<Uuid, Vec<RegieOrganization>> = HashMap::new();
    for o in lecture::organisations_des_seances(&mut *tx, &ids).await? {
        organisations
            .entry(o.session_id)
            .or_default()
            .push(RegieOrganization {
                id: o.id,
                name: o.name,
                acronym: o.acronym,
                logo: o.logo,
            });
    }
    let mut pieces: HashMap<Uuid, Vec<RegieDocument>> = HashMap::new();
    for p in lecture::pieces_des_seances(&mut *tx, &ids).await? {
        pieces.entry(p.session_id).or_default().push(RegieDocument {
            id: p.id,
            proposal_id: p.proposal_id,
            title: p.title,
            filename: p.filename,
            mime_type: p.mime_type,
            byte_size: p.byte_size,
            url: p.url,
            uploaded_at: p.uploaded_at,
        });
    }

    let gestes = lecture::gestes_du_jour(&mut *tx, event_id, jour).await?;
    let annulations: Vec<Uuid> = gestes
        .iter()
        .filter(|g| g.action_code == RegieActionKind::SessionCancelled.as_str())
        .map(|g| g.id)
        .collect();
    let envois: HashMap<Uuid, EnvoiDesCourriels> =
        lecture::envois_des_annulations(&mut *tx, &annulations)
            .await?
            .into_iter()
            .map(|e| (e.action_id, e))
            .collect();
    let messages = lecture::messages_du_jour(&mut *tx, event_id, jour).await?;
    let direct = lecture::direct_en_cours(&mut *tx, None).await?;
    let raisons = lecture::raisons_dannulation(&mut *tx).await?;
    tx.commit().await?;

    let sessions = lignes
        .into_iter()
        .map(|l| {
            // Les gestes sont rangés du plus récent au plus ancien.
            let annulation = gestes.iter().find(|g| {
                g.session_id == l.id
                    && g.action_code == RegieActionKind::SessionCancelled.as_str()
                    && g.undone_at.is_none()
            });
            let dernier = gestes
                .iter()
                .find(|g| g.session_id == l.id && defaisable(g, maintenant))
                .map(|g| g.id);
            let reprise = (l.status == "completed")
                .then_some(l.live_ended_at)
                .flatten()
                .map(|fin| fin + Duration::minutes(REPRISE_MINUTES))
                .filter(|limite| *limite >= maintenant);
            RegieSession {
                session_id: l.id,
                title: l.title,
                starts_at: l.starts_at,
                ends_at: l.ends_at,
                is_published: l.is_published,
                is_streamed: l.is_streamed,
                is_joint: l.is_joint,
                room_name: l.room_name,
                cover: l.cover,
                organizations: organisations.remove(&l.id).unwrap_or_default(),
                registrants: l.registrants,
                presence: RegiePresence {
                    state: match l.presence {
                        Some(true) => PresenceState::Confirmed,
                        Some(false) => PresenceState::Absent,
                        None => PresenceState::Unknown,
                    },
                    noted_at: l.presence_noted_at,
                    noted_by: l.presence_noted_by,
                },
                meeting: l.meeting_status.map(|status| RegieMeeting {
                    status,
                    sync_status: l.meeting_sync_status.unwrap_or_default(),
                    ready: l.meeting_ready,
                    present_count: l.meeting_present,
                }),
                documents: pieces.remove(&l.id).unwrap_or_default(),
                live_started_at: l.live_started_at,
                live_ended_at: l.live_ended_at,
                resumable_until: reprise,
                cancellation: (l.status == "cancelled").then(|| RegieCancellation {
                    cancelled_at: l.cancelled_at,
                    reason_code: l.reason_code,
                    reason_label: l.reason_label,
                    note: l.cancellation_note,
                    action_id: annulation.map(|g| g.id),
                    notice: annulation
                        .map(|g| etat_des_courriels(g, envois.get(&g.id)))
                        .unwrap_or(RegieNoticeStatus {
                            state: NoticeState::None,
                            due_at: None,
                            sent_at: None,
                            recipients: 0,
                        }),
                }),
                last_action_id: dernier,
                status: l.status,
            }
        })
        .collect::<Vec<_>>();

    let mut journal: Vec<RegieJournalEntry> = gestes
        .iter()
        .filter_map(|g| {
            let kind = RegieActionKind::parse(&g.action_code)?;
            Some(RegieJournalEntry {
                id: g.id,
                kind: JournalKind::from(kind),
                session_id: Some(g.session_id),
                session_title: Some(g.session_title.clone()),
                session_starts_at: Some(g.session_starts_at),
                incident_message: None,
                incident_kind_code: None,
                acted_at: g.acted_at,
                actor_name: g.actor_name.clone(),
                actor_is_me: g.acted_by == Some(perimetre.person_id),
                undone_at: g.undone_at,
                undone_by: g.undone_by.clone(),
                can_undo: defaisable(g, maintenant),
                notice: (kind == RegieActionKind::SessionCancelled)
                    .then(|| etat_des_courriels(g, envois.get(&g.id))),
            })
        })
        .collect();
    journal.extend(messages.into_iter().map(|m| RegieJournalEntry {
        id: m.id,
        kind: JournalKind::IncidentPublished,
        session_id: m.session_id,
        session_title: m.session_title,
        session_starts_at: m.session_starts_at,
        incident_message: Some(m.message),
        incident_kind_code: Some(m.kind_code),
        acted_at: m.published_at,
        actor_name: m.publisher_name,
        actor_is_me: m.published_by == Some(perimetre.person_id),
        undone_at: m.unpublished_at,
        undone_by: m.unpublisher_name,
        can_undo: m.still_shown,
        notice: None,
    }));
    journal.sort_by_key(|e| std::cmp::Reverse(e.acted_at));

    let live_elsewhere = direct
        .filter(|d| !sessions.iter().any(|s| s.session_id == d.session_id))
        .map(|d| RegieLiveElsewhere {
            session_id: d.session_id,
            same_event: d.event_id == event_id,
            title: (d.event_id == event_id).then_some(d.title),
        });

    Ok(RegieScreen {
        event_id,
        event_title: edition.title,
        event_acronym: edition.acronym,
        timezone: edition.timezone,
        zone_label: edition.city,
        days: jours,
        day: jour,
        today: edition.today,
        now: maintenant,
        live_elsewhere,
        sessions,
        journal,
        reasons: raisons
            .into_iter()
            .map(|r| RegieReason {
                code: r.code,
                label: r.label,
                description: r.description,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn le_jour_par_defaut_est_aujourdhui_puis_le_prochain_puis_le_premier() {
        let jours = [date!(2026 - 11 - 12), date!(2026 - 11 - 13)];
        assert_eq!(
            jour_par_defaut(&jours, date!(2026 - 11 - 13)),
            date!(2026 - 11 - 13)
        );
        assert_eq!(
            jour_par_defaut(&jours, date!(2026 - 11 - 01)),
            date!(2026 - 11 - 12)
        );
        assert_eq!(
            jour_par_defaut(&jours, date!(2026 - 11 - 20)),
            date!(2026 - 11 - 12)
        );
        assert_eq!(
            jour_par_defaut(&[], date!(2026 - 11 - 20)),
            date!(2026 - 11 - 20)
        );
    }
}
