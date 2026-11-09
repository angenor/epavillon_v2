//! Les intervenants en ligne (08/11), `075_programme_sessions.sql` § 2 bis.
//!
//! Deux portes : le lien d'inscription de l'activité (sans compte, jeton en
//! en-tête) et la régie du jour (`programme.session.schedule`). L'une et
//! l'autre posent la ligne d'intervenant puis confient au module Live la place
//! Zoom et le courriel du lien (`live.speaker_places`) ; le lien d'inscription
//! part au porteur par le module Engagement.

use contracts::engagement::{SpeakerSignupLinkEmail, EMAIL_QUEUE, SEND_SPEAKER_SIGNUP_LINK};
use contracts::live::{SpeakerPlaces, LIVE_QUEUE, SPEAKER_PLACES};
use kernel::auth::Perimeter;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::jobs::{self, NewJob};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::SessionId;
use crate::domain::online_speakers::{
    lien_dinscription, OnlineSpeakerCreated, OnlineSpeakerInput, RegieSpeaker, RegieSpeakers,
    SpeakerLinkInvite, SpeakerLinksSent, SOURCE_LIEN, SOURCE_REGIE,
};
use crate::repo::cross::intervenants::{self as lecture, SeanceDuLien};
use crate::repo::online_speakers as repo;
use crate::repo::people::{self, IdentiteSaisie};
use crate::service::perimeter::Cible;
use crate::service::regie::garder;
use crate::state::ProgrammeState;

fn invitation_de(s: SeanceDuLien) -> SpeakerLinkInvite {
    SpeakerLinkInvite {
        session_id: s.id,
        title: s.title,
        event_title: s.event_title,
        starts_at: s.starts_at,
        ends_at: s.ends_at,
        timezone: s.timezone,
        city: s.city,
    }
}

/// La séance du lien, s'il sert encore. Inconnu, activité finie, annulée ou
/// non diffusée : 404, indiscernables.
async fn seance_ouverte(state: &ProgrammeState, jeton: &str) -> Result<SeanceDuLien> {
    lecture::seance_par_jeton(state.pool(), jeton)
        .await?
        .filter(|s| s.ouverte(OffsetDateTime::now_utc()))
        .ok_or_else(ApiError::not_found)
}

pub async fn invitation(state: &ProgrammeState, jeton: &str) -> Result<SpeakerLinkInvite> {
    Ok(invitation_de(seance_ouverte(state, jeton).await?))
}

/// La place Zoom et le courriel du lien, confiés au module Live. La clé porte
/// l'instant : deux demandes, deux envois ; un rejeu de la même, aucun.
async fn demander_les_places(
    conn: &mut PgConnection,
    session_id: Uuid,
    speaker_ids: Option<Vec<Uuid>>,
) -> Result<()> {
    let charge = SpeakerPlaces {
        session_id,
        speaker_ids,
        auto: false,
    };
    let job = NewJob {
        queue: LIVE_QUEUE,
        ..NewJob::new(
            SPEAKER_PLACES,
            serde_json::to_value(&charge).map_err(ApiError::internal)?,
        )
    }
    .idempotent(format!(
        "{session_id}:{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
    ));
    jobs::enqueue(conn, job).await?;
    Ok(())
}

async fn poser_intervenant(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session_id: Uuid,
    charge: &OnlineSpeakerInput,
    source: &str,
) -> Result<OnlineSpeakerCreated> {
    let saisie = charge.verifier()?;
    let mut tx = state.db().write(ctx).await?;
    let personne = people::trouver_ou_creer(
        &mut tx,
        IdentiteSaisie {
            email: &saisie.email,
            first_name: &saisie.first_name,
            last_name: &saisie.last_name,
            civility: saisie.civility.as_deref(),
        },
    )
    .await?;
    let speaker_id = repo::poser(
        &mut tx,
        session_id,
        personne,
        &saisie,
        source,
        source == SOURCE_REGIE,
    )
    .await?;
    demander_les_places(&mut tx, session_id, Some(vec![speaker_id])).await?;
    tx.commit().await?;
    Ok(OnlineSpeakerCreated { speaker_id })
}

pub async fn sinscrire_par_le_lien(
    state: &ProgrammeState,
    ctx: &RequestContext,
    jeton: &str,
    charge: OnlineSpeakerInput,
) -> Result<OnlineSpeakerCreated> {
    let seance = seance_ouverte(state, jeton).await?;
    if !charge.consent {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationConsentRequired,
            "Cochez la case d'accord pour figurer parmi les intervenants de l'activité.",
        )
        .field("consent"));
    }
    poser_intervenant(state, ctx, seance.id, &charge, SOURCE_LIEN).await
}

/// Une séance de la régie qui se suit en ligne et n'est pas annulée.
async fn seance_de_regie(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<SeanceDuLien> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let seance = lecture::seance_par_id(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if seance.status == "cancelled" {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationNotAccepted,
            "Cette activité est annulée.",
        ));
    }
    if !seance.is_streamed {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationNotAccepted,
            "Cette activité n'est pas diffusée en ligne : elle n'a pas de réunion Zoom.",
        ));
    }
    Ok(seance)
}

pub async fn liste(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<RegieSpeakers> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let seance = lecture::seance_par_id(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    lire(state, &seance).await
}

async fn lire(state: &ProgrammeState, seance: &SeanceDuLien) -> Result<RegieSpeakers> {
    let pool = state.pool();
    let speakers = lecture::intervenants_de(pool, seance.id)
        .await?
        .into_iter()
        .map(|i| RegieSpeaker {
            speaker_id: i.id,
            display_name: i.display_name,
            job_title: i.job_title,
            organization: i.organization,
            email: i.email,
            source: i.source,
            is_listed: i.is_listed,
            created_at: i.created_at,
            link_sent_at: i.link_sent_at,
        })
        .collect();
    let resume = lecture::resumes(pool, &[seance.id]).await?;
    Ok(RegieSpeakers {
        session_id: seance.id,
        speakers,
        signup_url: lien_dinscription(&state.config().app_public_url, &seance.token),
        owner_emails: lecture::porteurs(pool, seance.id).await?,
        owner_link_sent_at: resume.first().and_then(|r| r.owner_link_sent_at),
    })
}

pub async fn inscrire_par_la_regie(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
    charge: OnlineSpeakerInput,
) -> Result<OnlineSpeakerCreated> {
    seance_de_regie(state, perimetre, session_id).await?;
    poser_intervenant(state, ctx, session_id, &charge, SOURCE_REGIE).await
}

pub async fn afficher(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
    speaker_id: Uuid,
) -> Result<RegieSpeakers> {
    garder(state, perimetre, Cible::Seance(SessionId::from(session_id))).await?;
    let mut tx = state.db().write(ctx).await?;
    if !repo::afficher(&mut tx, session_id, speaker_id).await? {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    let seance = lecture::seance_par_id(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    lire(state, &seance).await
}

pub async fn renvoyer_les_liens(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<SpeakerLinksSent> {
    seance_de_regie(state, perimetre, session_id).await?;
    let joignables = lecture::intervenants_de(state.pool(), session_id)
        .await?
        .iter()
        .filter(|i| i.email.is_some())
        .count() as i64;
    if joignables == 0 {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationNotAccepted,
            "Aucun intervenant de cette activité n'a d'adresse courriel : inscrivez-les d'abord.",
        ));
    }
    let mut tx = state.db().write(ctx).await?;
    demander_les_places(&mut tx, session_id, None).await?;
    tx.commit().await?;
    Ok(SpeakerLinksSent {
        recipients: joignables,
    })
}

pub async fn envoyer_au_porteur(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<SpeakerLinksSent> {
    seance_de_regie(state, perimetre, session_id).await?;
    let porteurs = lecture::porteurs(state.pool(), session_id).await?;
    if porteurs.is_empty() {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationNotAccepted,
            "Cette activité n'a pas de porteur à qui écrire : copiez le lien d'inscription et transmettez-le vous-même.",
        ));
    }
    let charge = SpeakerSignupLinkEmail {
        session_id,
        preview_to: None,
    };
    let job = NewJob {
        queue: EMAIL_QUEUE,
        ..NewJob::new(
            SEND_SPEAKER_SIGNUP_LINK,
            serde_json::to_value(&charge).map_err(ApiError::internal)?,
        )
    }
    .idempotent(format!(
        "{session_id}:{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
    ));
    let mut tx = state.db().write(ctx).await?;
    jobs::enqueue(&mut tx, job).await?;
    tx.commit().await?;
    Ok(SpeakerLinksSent {
        recipients: porteurs.len() as i64,
    })
}
