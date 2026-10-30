//! La page publique d'un fil (journée spéciale ou atelier préparatoire) en
//! une réponse, et le placement d'une activité dans son déroulé (30/10).
//!
//! # Un fil non publié n'existe pas
//!
//! Adresse inconnue, fil sans page ouverte, édition en brouillon ou annulée :
//! le même 404. Les distinguer dirait au public qu'une journée se prépare.
//!
//! # Le déroulé est fusionné ici, une fois
//!
//! Activités **publiées** du fil et moments sans activité, triés par heure (un
//! moment passe avant une activité de même heure, puis le rang du
//! rattachement). Les heures de la journée et de chaque partie **se lisent sur
//! ce déroulé** : aucune n'est saisie, aucune ne peut le contredire.

use std::collections::HashMap;

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::pg_error;
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{EventId, SessionId};
use crate::domain::track_page::{
    PublicTrackActivity, PublicTrackEdition, PublicTrackEntry, PublicTrackEntryBody,
    PublicTrackFeature, PublicTrackHeader, PublicTrackMoment, PublicTrackPage, PublicTrackPart,
    PublicScheduleTrack, PublicScheduleTrackMoment, PublicTrackVenue, TrackSessionPlacement,
    TrackSessionPlacementPayload,
};
use crate::repo::{cross, public_schedule, session_parts, track_page};
use crate::state::ProgrammeState;

/// Les fils publiés d'une édition et leurs moments, sans page lue : ce que
/// `/programmations` regroupe en cadres (journée spéciale) ou en encadré
/// (atelier). Deux lectures, quel que soit le nombre de fils.
pub async fn fils_de_ledition(pool: &PgPool, event_id: EventId) -> Result<Vec<PublicScheduleTrack>> {
    let fils = cross::fils_publies_de_ledition(pool, event_id.0, None).await?;
    if fils.is_empty() {
        return Ok(Vec::new());
    }
    let mut moments: HashMap<String, Vec<PublicScheduleTrackMoment>> = HashMap::new();
    for m in cross::moments_des_fils_publies(pool, event_id.0).await? {
        moments.entry(m.track_slug).or_default().push(PublicScheduleTrackMoment {
            starts_at: m.starts_at,
            ends_at: m.ends_at,
            label: m.label,
        });
    }

    Ok(fils
        .into_iter()
        .map(|summary| PublicScheduleTrack {
            moments: moments.remove(&summary.slug).unwrap_or_default(),
            summary,
        })
        .collect())
}

/// La page publique d'un fil publié, par les adresses de l'édition et du fil.
pub async fn page(pool: &PgPool, event_slug: &str, track_slug: &str) -> Result<PublicTrackPage> {
    let fil = cross::fil_publie(pool, event_slug, track_slug)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let parties = cross::parties_du_fil(pool, fil.id).await?;
    let moments = cross::moments_du_fil(pool, fil.id).await?;
    let partners = cross::partenaires_du_fil(pool, fil.id).await?;
    let rattachements = track_page::rattachements_publies(pool, fil.id).await?;

    let ids: Vec<Uuid> = rattachements.iter().map(|r| r.session_id).collect();
    let lignes = public_schedule::des_seances(pool, EventId(fil.event_id), &ids).await?;
    let mut intervenants = par_seance(session_parts::intervenants_publics_de(pool, &ids).await?);
    let mut organisations =
        par_seance(session_parts::organisations_publiques_de(pool, &ids).await?);
    let placement: HashMap<Uuid, (Option<Uuid>, bool, i16)> = rattachements
        .iter()
        .map(|r| (r.session_id, (r.part_id, r.is_highlight, r.sort_order)))
        .collect();

    // (début, rang de nature, rang du rattachement) : la clé du tri.
    let mut lignes_du_deroule: Vec<((OffsetDateTime, u8, i16), PublicTrackEntry)> = Vec::new();

    for m in moments {
        lignes_du_deroule.push((
            (m.starts_at, 0, 0),
            PublicTrackEntry {
                starts_at: m.starts_at,
                ends_at: m.ends_at,
                part_id: m.part_id,
                body: PublicTrackEntryBody::Moment {
                    moment: PublicTrackMoment {
                        id: m.id,
                        label: m.label,
                        description: m.description,
                    },
                },
            },
        ));
    }

    for session in lignes {
        let (part_id, is_highlight, rang) = placement
            .get(&session.id)
            .copied()
            .unwrap_or((None, false, 0));
        let id = session.id;
        lignes_du_deroule.push((
            (session.starts_at, 1, rang),
            PublicTrackEntry {
                starts_at: session.starts_at,
                ends_at: Some(session.ends_at),
                part_id,
                body: PublicTrackEntryBody::Activity {
                    activity: PublicTrackActivity {
                        is_highlight,
                        speakers: intervenants.remove(&id).unwrap_or_default(),
                        organizations: organisations.remove(&id).unwrap_or_default(),
                        session,
                    },
                },
            },
        ));
    }

    lignes_du_deroule.sort_by_key(|(cle, _)| *cle);
    let agenda: Vec<PublicTrackEntry> = lignes_du_deroule.into_iter().map(|(_, e)| e).collect();

    let parts = parties
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let contenu: Vec<&PublicTrackEntry> =
                agenda.iter().filter(|e| e.part_id == Some(p.id)).collect();
            PublicTrackPart {
                id: p.id,
                number: i as i32 + 1,
                label: p.label,
                title: p.title,
                summary: p.summary,
                starts_at: contenu.iter().map(|e| e.starts_at).min(),
                ends_at: contenu.iter().map(|e| fin(e)).max(),
            }
        })
        .collect();

    let est_activite = |e: &&PublicTrackEntry| matches!(e.body, PublicTrackEntryBody::Activity { .. });
    let premiere_activite = agenda.iter().filter(est_activite).map(|e| e.starts_at).min();
    let starts_at = premiere_activite.or_else(|| agenda.iter().map(|e| e.starts_at).min());
    let ends_at = agenda.iter().map(fin).max();
    let opens_at = agenda
        .iter()
        .filter(|e| matches!(e.body, PublicTrackEntryBody::Moment { .. }))
        .map(|e| e.starts_at)
        .min()
        .filter(|debut| premiere_activite.is_some_and(|a| *debut < a));

    let other_tracks = cross::fils_publies_de_ledition(pool, fil.event_id, Some(fil.id)).await?;

    let feature = fil.feature_title.map(|title| PublicTrackFeature {
        kicker: fil.feature_kicker,
        title,
        body: fil.feature_body,
        link_label: fil.feature_link_label,
        link_url: fil.feature_link_url,
        visual: fil.feature_visual,
    });

    let venue = match (fil.venue_id, fil.venue_name, fil.venue_kind) {
        (Some(id), Some(name), Some(kind)) => Some(PublicTrackVenue {
            id,
            name,
            kind,
            address: fil.venue_address,
            map_url: fil.venue_map_url,
            area: fil.venue_area,
            access_note: fil.venue_access_note,
            latitude: fil.venue_latitude,
            longitude: fil.venue_longitude,
        }),
        _ => None,
    };

    Ok(PublicTrackPage {
        edition: PublicTrackEdition {
            id: fil.event_id,
            slug: fil.event_slug,
            title: fil.event_title,
            acronym: fil.event_acronym,
            timezone: fil.timezone,
            starts_at: fil.event_starts_at,
            ends_at: fil.event_ends_at,
            city: fil.city,
            country_name: fil.country_name,
        },
        track: PublicTrackHeader {
            id: fil.id,
            slug: fil.slug,
            code: fil.code,
            kind: fil.kind,
            title: fil.title,
            subtitle: fil.subtitle,
            description: fil.description,
            tagline: fil.tagline,
            starts_on: fil.starts_on,
            ends_on: fil.ends_on,
            color_hex: fil.color_hex,
            themes: fil.themes,
        },
        venue,
        is_off_site: fil.is_off_site,
        cover: fil.cover,
        documents: en_liste(fil.documents),
        feature,
        target_audiences: en_liste(fil.target_audiences),
        support_note: fil.support_note,
        partners,
        parts,
        agenda,
        starts_at,
        ends_at,
        opens_at,
        other_tracks,
    })
}

/// Placer une activité dans le déroulé d'un fil : partie, temps fort, rang. La
/// séance a été gardée par la route (périmètre, permission de planifier) ;
/// une séance non rattachée à ce fil est introuvable.
pub async fn placer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session: SessionId,
    track: Uuid,
    payload: TrackSessionPlacementPayload,
) -> Result<TrackSessionPlacement> {
    let mut tx = state.db().write(ctx).await?;
    let place = track_page::placer(&mut tx, session.as_uuid(), track, &payload)
        .await
        .map_err(|e| pg_error::translate(&e))?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;
    Ok(place)
}

/// Fin d'une ligne : sa fin, à défaut son début (la clôture n'a qu'une heure).
fn fin(e: &PublicTrackEntry) -> OffsetDateTime {
    e.ends_at.unwrap_or(e.starts_at)
}

fn en_liste(v: Value) -> Vec<Value> {
    match v {
        Value::Array(elements) => elements,
        _ => Vec::new(),
    }
}

fn par_seance(lignes: Vec<Value>) -> HashMap<Uuid, Vec<Value>> {
    let mut groupes: HashMap<Uuid, Vec<Value>> = HashMap::new();
    for ligne in lignes {
        if let Some(id) = ligne
            .get("session_id")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            groupes.entry(id).or_default().push(ligne);
        }
    }
    groupes
}
