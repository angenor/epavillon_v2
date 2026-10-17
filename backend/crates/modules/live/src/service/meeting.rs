//! Les réunions Zoom d'une séance : ce que le public en voit, l'entrée d'une
//! personne inscrite, et le back-office.
//!
//! Appelé directement par les tests : les règles d'accès vivent ici et non
//! dans les gestionnaires Actix (voir `tests/commun`).

use kernel::auth::{has_permission, Perimeter, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::meeting::{
    AdminSessionMeeting, MeetingAccess, MeetingSyncInput, PublicSessionMeeting,
    FERMETURE_APRES_MINUTES, OUVERTURE_AVANT_MINUTES,
};
use crate::repo::cross::programme;
use crate::repo::{lecture, meetings};
use crate::service::inscrits;
use crate::state::LiveState;
use crate::zoom::{self, jwt};

pub const MEETING_MANAGE: &str = "live.meeting.manage";

/// Séance publiée et réunion programmée, en cours ou terminée ; sinon 404, la
/// même réponse qu'une séance inconnue.
pub async fn publique(state: &LiveState, session_id: Uuid) -> Result<PublicSessionMeeting> {
    let mut tx = lecture(state.pool()).await?;
    let seance = programme::seance(&mut tx, session_id)
        .await?
        .filter(|s| s.is_published)
        .ok_or_else(ApiError::not_found)?;
    let reunion = meetings::reunion_publique(&mut tx, session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;

    Ok(PublicSessionMeeting {
        provider: reunion.provider,
        status: reunion.status,
        opens_at: seance.starts_at - Duration::minutes(OUVERTURE_AVANT_MINUTES),
        starts_at: seance.starts_at,
        ends_at: seance.ends_at,
        closes_at: seance.ends_at + Duration::minutes(FERMETURE_APRES_MINUTES),
    })
}

/// L'entrée d'une personne inscrite. Ordre des refus : Zoom configuré, séance
/// connue, séance ni annulée ni reportée, inscription, salle ouverte, place prête.
pub async fn acces(
    state: &LiveState,
    person_id: Uuid,
    session_id: Uuid,
    maintenant: OffsetDateTime,
) -> Result<MeetingAccess> {
    let zoom = state.zoom()?;
    let mut tx = lecture(state.pool()).await?;

    let seance = programme::seance(&mut tx, session_id)
        .await?
        .filter(|s| s.is_published)
        .ok_or_else(ApiError::not_found)?;
    if matches!(seance.status.as_str(), "cancelled" | "postponed") {
        return Err(ApiError::new(ErrorCode::LiveMeetingCancelled));
    }

    let registration_id = programme::inscription_active_de(&mut tx, session_id, person_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::LiveMeetingRegistrationRequired))?;

    let ouverture = seance.starts_at - Duration::minutes(OUVERTURE_AVANT_MINUTES);
    let fermeture = seance.ends_at + Duration::minutes(FERMETURE_APRES_MINUTES);
    if maintenant < ouverture || maintenant > fermeture {
        return Err(ApiError::new(ErrorCode::LiveMeetingNotOpen));
    }

    let place = meetings::place(&mut tx, registration_id).await?;
    tx.commit().await?;

    let not_ready = || ApiError::new(ErrorCode::LiveMeetingNotReady);
    let place = place.ok_or_else(not_ready)?;
    let numero = place.external_id.ok_or_else(not_ready)?;
    let lien = place.personal_join_url.ok_or_else(not_ready)?;
    let password = match place.passcode_encrypted {
        Some(octets) => zoom.coffre.dechiffrer(&octets)?,
        None => String::new(),
    };

    let signe = jwt::signer(
        &zoom.config.sdk_key,
        zoom.config.sdk_secret.expose(),
        &numero,
        0,
        maintenant.unix_timestamp(),
    );

    Ok(MeetingAccess {
        meeting_number: numero,
        password,
        registrant_token: zoom::jeton_du_lien(&lien),
        user_name: place.display_name,
        user_email: place.email,
        sdk_key: zoom.config.sdk_key.clone(),
        signature: signe.signature,
        expires_at: OffsetDateTime::from_unix_timestamp(signe.expire_a)
            .map_err(ApiError::internal)?,
        join_url: lien,
    })
}

/// Périmètre de l'édition de la séance, puis permission sur cette édition.
async fn autoriser(state: &LiveState, perimetre: &Perimeter, session_id: Uuid) -> Result<()> {
    let mut conn = state.pool().acquire().await?;
    let seance = programme::seance(&mut conn, session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    perimetre.ensure(seance.event_id)?;
    if !has_permission(
        state.pool(),
        perimetre.person_id,
        MEETING_MANAGE,
        Scope::Event(seance.event_id),
    )
    .await?
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

pub async fn lire_admin(
    state: &LiveState,
    perimetre: &Perimeter,
    session_id: Uuid,
) -> Result<AdminSessionMeeting> {
    autoriser(state, perimetre, session_id).await?;
    let mut conn = state.pool().acquire().await?;
    meetings::tableau(&mut conn, session_id)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// Crée ou rattache la réunion, (re)crée un inscrit Zoom par inscription qui a
/// une place, et relance la synchronisation. Rejouable sans dommage.
pub async fn reconstruire(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    session_id: Uuid,
    entree: &MeetingSyncInput,
) -> Result<AdminSessionMeeting> {
    autoriser(state, perimetre, session_id).await?;
    state.zoom()?;

    let external_id = match entree.external_id.as_deref().map(normaliser_numero) {
        Some(Some(n)) => Some(n),
        Some(None) => {
            return Err(ApiError::validation(
                "Le numéro de réunion Zoom ne contient que des chiffres.",
                "external_id",
            ))
        }
        None => None,
    };

    let mut tx = state.db().write(ctx).await?;
    let seance = programme::seance(&mut tx, session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let existante = meetings::reunion_active(&mut tx, session_id).await?;
    if let Some(n) = &external_id {
        let deja = existante.as_ref().and_then(|r| r.external_id.as_deref()) == Some(n.as_str());
        if !deja {
            // Rattacher une réunion existante du compte donne son code d'accès :
            // réservé à qui gère les réunions sur toute la plateforme.
            if !has_permission(
                state.pool(),
                perimetre.person_id,
                MEETING_MANAGE,
                Scope::Global,
            )
            .await?
            {
                return Err(ApiError::forbidden());
            }
            if existante
                .as_ref()
                .is_some_and(|r| r.sync_status == "in_progress")
            {
                return Err(ApiError::with_message(
                    ErrorCode::Conflict,
                    "La réunion est en cours de synchronisation avec Zoom. Réessayez dans une minute.",
                ));
            }
        }
        if meetings::external_id_pris(&mut tx, n, existante.as_ref().map(|r| r.id)).await? {
            return Err(ApiError::new(ErrorCode::LiveMeetingExternalIdTaken));
        }
    }

    let (meeting_id, creee) = match existante {
        Some(r) => {
            if let Some(n) = &external_id {
                if r.external_id.as_deref() != Some(n.as_str()) {
                    meetings::rattacher(&mut tx, r.id, n).await?;
                }
            }
            (r.id, false)
        }
        None => (
            inscrits::assurer_reunion(&mut tx, &seance, external_id.as_deref()).await?,
            true,
        ),
    };

    for registration_id in programme::inscriptions_actives(&mut tx, session_id).await? {
        inscrits::synchroniser(&mut tx, registration_id).await?;
    }

    // Une réunion qu'on vient d'insérer est déjà en file par son déclencheur.
    if !creee {
        meetings::relancer_reunion(&mut tx, meeting_id).await?;
    }

    let tableau = meetings::tableau(&mut tx, session_id)
        .await?
        .ok_or_else(|| ApiError::internal("réunion introuvable après reconstruction"))?;
    tx.commit().await?;
    Ok(tableau)
}

/// Un numéro de réunion Zoom saisi à la main : espaces et tirets tolérés.
fn normaliser_numero(brut: &str) -> Option<String> {
    let chiffres: String = brut
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    (!chiffres.is_empty() && chiffres.len() <= 20 && chiffres.chars().all(|c| c.is_ascii_digit()))
        .then_some(chiffres)
}
