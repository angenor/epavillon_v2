//! L'éditeur de la page publique d'un fil (30/10) : lieu, accroche, publics,
//! partenaires et mention, encart mis en avant, parties, moments.
//!
//! Le fil a déjà été remonté à son édition et gardé par la route (périmètre puis
//! permission, règle n° 8). Chaque écriture se fait dans **une** transaction et
//! rend les réglages entiers, relus après validation.
//!
//! Placer une activité dans une partie n'est pas ici : le rattachement
//! séance–fil appartient au module Programmation, qui l'écrit.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::pg_error;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ids::TrackId;
use crate::domain::track_page::{
    TrackMomentPayload, TrackPagePayload, TrackPageSettings, TrackPartPayload,
};
use crate::repo::{cross_page, track_page};
use crate::state::EventState;

/// Les réglages de la page d'un fil.
pub async fn lire(pool: &PgPool, track: TrackId) -> Result<TrackPageSettings> {
    let colonnes = track_page::colonnes(pool, track)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let target_audiences = match colonnes.target_audiences {
        serde_json::Value::Array(publics) => publics,
        _ => Vec::new(),
    };

    Ok(TrackPageSettings {
        track_id: track.as_uuid(),
        event_id: colonnes.event_id,
        venue_id: colonnes.venue_id,
        tagline: colonnes.tagline,
        target_audiences,
        support_note: colonnes.support_note,
        feature_kicker: colonnes.feature_kicker,
        feature_title: colonnes.feature_title,
        feature_body: colonnes.feature_body,
        feature_link_label: colonnes.feature_link_label,
        feature_link_url: colonnes.feature_link_url,
        parts: track_page::parties(pool, track).await?,
        moments: track_page::moments(pool, track).await?,
        partners: cross_page::partenaires(pool, track).await?,
        sessions: cross_page::composition(pool, track).await?,
    })
}

/// Écrire les colonnes de la page et **remplacer** les partenaires.
pub async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    mut payload: TrackPagePayload,
) -> Result<TrackPageSettings> {
    // Le français est exigé sur chaque public, comme sur tout texte multilingue :
    // le dire ici plutôt que laisser la base refuser sans nommer le champ.
    if payload.target_audiences.iter().any(|p| !a_son_francais(p)) {
        return Err(ApiError::validation(
            "Chaque public cible doit avoir son libellé en français.",
            "target_audiences",
        ));
    }

    // Une organisation une fois par fil : on garde sa première place.
    let mut vues = std::collections::HashSet::new();
    payload.partners.retain(|p| vues.insert(p.organization_id));

    let mut tx = state.db().write(ctx).await?;

    let codes: Vec<String> = payload.partners.iter().map(|p| p.role_code.clone()).collect();
    let inconnus = track_page::roles_inconnus(&mut tx, &codes).await?;
    if let Some(code) = inconnus.first() {
        return Err(ApiError::validation(
            format!("Rôle de partenaire inconnu : « {code} »."),
            "partners",
        ));
    }

    match track_page::ecrire(&mut tx, track, &payload).await {
        Ok(true) => {}
        Ok(false) => return Err(ApiError::not_found()),
        Err(e) => return Err(pg_error::translate(&e)),
    }
    track_page::remplacer_partenaires(&mut tx, track, &payload.partners)
        .await
        .map_err(|e| pg_error::translate(&e))?;

    tx.commit().await?;
    lire(state.pool(), track).await
}

/// Ajouter une partie.
pub async fn creer_partie(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    payload: TrackPartPayload,
) -> Result<TrackPageSettings> {
    let mut tx = state.db().write(ctx).await?;
    track_page::creer_partie(&mut tx, track, &payload)
        .await
        .map_err(|e| pg_error::translate(&e))?;
    tx.commit().await?;
    lire(state.pool(), track).await
}

/// Modifier une partie **de ce fil** ; une partie d'un autre fil est introuvable.
pub async fn modifier_partie(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    part: Uuid,
    payload: TrackPartPayload,
) -> Result<TrackPageSettings> {
    let mut tx = state.db().write(ctx).await?;
    let trouvee = track_page::modifier_partie(&mut tx, track, part, &payload)
        .await
        .map_err(|e| pg_error::translate(&e))?;
    if !trouvee {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    lire(state.pool(), track).await
}

/// Retirer une partie : ses moments et ses activités restent, hors partie.
pub async fn supprimer_partie(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    part: Uuid,
) -> Result<TrackPageSettings> {
    let mut tx = state.db().write(ctx).await?;
    if !track_page::supprimer_partie(&mut tx, track, part).await? {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    lire(state.pool(), track).await
}

/// Ajouter un moment sans activité.
pub async fn creer_moment(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    payload: TrackMomentPayload,
) -> Result<TrackPageSettings> {
    exiger_un_libelle(&payload)?;
    let mut tx = state.db().write(ctx).await?;
    track_page::creer_moment(&mut tx, track, &payload)
        .await
        .map_err(|e| pg_error::translate(&e))?;
    tx.commit().await?;
    lire(state.pool(), track).await
}

/// Modifier un moment **de ce fil**.
pub async fn modifier_moment(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    moment: Uuid,
    payload: TrackMomentPayload,
) -> Result<TrackPageSettings> {
    exiger_un_libelle(&payload)?;
    let mut tx = state.db().write(ctx).await?;
    let trouve = track_page::modifier_moment(&mut tx, track, moment, &payload)
        .await
        .map_err(|e| pg_error::translate(&e))?;
    if !trouve {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    lire(state.pool(), track).await
}

pub async fn supprimer_moment(
    state: &EventState,
    ctx: &RequestContext,
    track: TrackId,
    moment: Uuid,
) -> Result<TrackPageSettings> {
    let mut tx = state.db().write(ctx).await?;
    if !track_page::supprimer_moment(&mut tx, track, moment).await? {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    lire(state.pool(), track).await
}

fn a_son_francais(texte: &serde_json::Value) -> bool {
    texte
        .get("fr")
        .and_then(|v| v.as_str())
        .is_some_and(|fr| !fr.trim().is_empty())
}

fn exiger_un_libelle(payload: &TrackMomentPayload) -> Result<()> {
    if a_son_francais(&payload.label) {
        Ok(())
    } else {
        Err(ApiError::validation(
            "Le libellé du moment est obligatoire en français.",
            "label",
        ))
    }
}
