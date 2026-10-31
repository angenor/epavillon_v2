//! Ce que le tableau de bord d'un dossier lit hors de son schéma (01/11) :
//! adresses et photos des personnes, salle et journées spéciales des séances,
//! état de la réunion Zoom, autres dossiers de l'organisation porteuse.
//!
//! **Même espace de noms que `cross/mod.rs`, mêmes règles** : lectures seules,
//! lecture n° 19 du tableau. Les adresses ne sortent que vers le back-office :
//! `PersonneAffichee` reste sans adresse pour les autres écrans.

use kernel::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Une personne à joindre : nom, fonction, adresse, photo.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PersonneAJoindre {
    pub person_id: Uuid,
    pub display_name: String,
    pub job_title: Option<String>,
    pub organization_name: Option<String>,
    pub email: String,
    /// `media.attached_image('identity', 'people', id, 'avatar')`, nulle sans photo servable.
    pub photo: Option<Value>,
}

pub async fn personnes_a_joindre<'e>(
    executor: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<PersonneAJoindre>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.display_name AS "display_name!", p.job_title,
                  p.primary_email::text AS "email!", o.legal_name AS "organization_name?",
                  media.attached_image('identity', 'people', p.id, 'avatar') AS photo
             FROM identity.people p
             LEFT JOIN org.organizations o ON o.id = p.primary_organization_id
            WHERE p.id = ANY($1)"#,
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PersonneAJoindre {
            person_id: l.id,
            display_name: l.display_name,
            job_title: l.job_title,
            organization_name: l.organization_name,
            email: l.email,
            photo: l.photo,
        })
        .collect())
}

/// Une journée spéciale à laquelle l'équipe a rattaché la séance.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JourneeDeLaSeance {
    pub id: Uuid,
    pub title: Value,
    pub color_hex: Option<String>,
}

/// La réunion de visioconférence de la séance, réduite à son état.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReunionDeLaSeance {
    pub provider: String,
    /// `live.meeting_status`.
    pub status: String,
    /// `live.sync_status` : `synced` veut dire « à jour chez le fournisseur ».
    pub sync_status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_synced_at: Option<OffsetDateTime>,
}

/// Une séance du dossier : le créneau retenu, là où il est tenu.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SeanceDuDossier {
    pub id: Uuid,
    pub sequence_number: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub timezone: String,
    /// `programme.session_status`.
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    /// Nulle tant que la séance n'est pas placée.
    pub room_id: Option<Uuid>,
    pub room_name: Option<Value>,
    pub is_streamed: bool,
    pub tracks: Vec<JourneeDeLaSeance>,
    /// Inscriptions confirmées : `registered`, `attended`, `no_show`.
    pub registration_count: i64,
    pub waitlist_count: i64,
    /// La réunion non annulée la plus récente, nulle sans réunion.
    pub meeting: Option<ReunionDeLaSeance>,
}

pub async fn seances_du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: Uuid,
) -> Result<Vec<SeanceDuDossier>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.sequence_number, s.starts_at, s.ends_at,
                  s.timezone::text AS "timezone!", s.status::text AS "status!",
                  s.published_at, s.room_id, r.name AS "room_name?", s.is_streamed,
                  (SELECT count(*) FROM programme.registrations g
                    WHERE g.session_id = s.id
                      AND g.status IN ('registered', 'attended', 'no_show')) AS "registration_count!",
                  (SELECT count(*) FROM programme.registrations g
                    WHERE g.session_id = s.id AND g.status = 'waitlisted') AS "waitlist_count!",
                  COALESCE((SELECT jsonb_agg(jsonb_build_object(
                                       'id', t.id, 'title', t.title, 'color_hex', t.color_hex)
                                     ORDER BY st.sort_order, t.title ->> 'fr')
                              FROM programme.session_tracks st
                              JOIN event.programme_tracks t ON t.id = st.track_id
                             WHERE st.session_id = s.id), '[]'::jsonb) AS "tracks!",
                  m.provider::text AS "meeting_provider?", m.status::text AS "meeting_status?",
                  m.sync_status::text AS "meeting_sync_status?",
                  m.last_synced_at AS "meeting_last_synced_at?"
             FROM programme.sessions s
             LEFT JOIN event.rooms r ON r.id = s.room_id
             LEFT JOIN LATERAL (
                 SELECT provider, status, sync_status, last_synced_at
                   FROM live.meetings
                  WHERE session_id = s.id AND status <> 'cancelled'
                  ORDER BY created_at DESC
                  LIMIT 1
             ) m ON true
            WHERE s.proposal_id = $1
            ORDER BY s.sequence_number"#,
        proposal_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| SeanceDuDossier {
            id: l.id,
            sequence_number: l.sequence_number,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            timezone: l.timezone,
            status: l.status,
            published_at: l.published_at,
            room_id: l.room_id,
            room_name: l.room_name,
            is_streamed: l.is_streamed,
            tracks: serde_json::from_value(l.tracks).unwrap_or_default(),
            registration_count: l.registration_count,
            waitlist_count: l.waitlist_count,
            meeting: match (l.meeting_provider, l.meeting_status, l.meeting_sync_status) {
                (Some(provider), Some(status), Some(sync_status)) => Some(ReunionDeLaSeance {
                    provider,
                    status,
                    sync_status,
                    last_synced_at: l.meeting_last_synced_at,
                }),
                _ => None,
            },
        })
        .collect())
}

/// Un autre dossier de l'organisation porteuse, dans le périmètre du lecteur.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AutreDossier {
    pub id: Uuid,
    pub reference_code: String,
    pub title: Value,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub event_id: Uuid,
    pub event_title: Value,
    pub event_acronym: Option<String>,
    /// Fuseau de l'édition du dossier : chaque date s'y lit.
    pub timezone: String,
    pub city: Option<String>,
    /// Début de la première séance placée et non annulée ; nul sinon.
    #[serde(with = "time::serde::rfc3339::option")]
    pub retained_starts_at: Option<OffsetDateTime>,
    #[serde(skip)]
    pub submitted_by: Uuid,
    /// Rempli par le service, depuis `personnes_a_joindre`.
    pub submitter: Option<PersonneAJoindre>,
}

/// **Règle n° 8** : une édition hors périmètre ne sort pas, même pour un
/// dossier de la même organisation.
pub async fn autres_dossiers<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
    sauf: Uuid,
    perimetre: &kernel::auth::AdminScope,
) -> Result<Vec<AutreDossier>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.reference_code, p.title, p.status::text AS "status!",
                  p.created_at, p.event_id, e.title AS event_title, e.acronym AS event_acronym,
                  e.timezone::text AS "timezone!", e.city, p.submitted_by,
                  (SELECT min(s.starts_at) FROM programme.sessions s
                    WHERE s.proposal_id = p.id AND s.room_id IS NOT NULL
                      AND s.status <> 'cancelled') AS retained_starts_at
             FROM programme.proposals p
             JOIN event.events e ON e.id = p.event_id
            WHERE p.organization_id = $1 AND p.id <> $2 AND p.deleted_at IS NULL
              AND ($3 OR p.event_id = ANY($4))
            ORDER BY p.created_at DESC
            LIMIT 100"#,
        organization_id,
        sauf,
        perimetre.is_global,
        &perimetre.event_ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AutreDossier {
            id: l.id,
            reference_code: l.reference_code,
            title: l.title,
            status: l.status,
            created_at: l.created_at,
            event_id: l.event_id,
            event_title: l.event_title,
            event_acronym: l.event_acronym,
            timezone: l.timezone,
            city: l.city,
            retained_starts_at: l.retained_starts_at,
            submitted_by: l.submitted_by,
            submitter: None,
        })
        .collect())
}

/// Le logo et le type de l'organisation porteuse, pour la carte latérale.
pub struct HabillageDeLaPorteuse {
    pub logo: Option<Value>,
    pub type_label: Option<Value>,
    pub country_name: Option<Value>,
}

pub async fn habillage_de_la_porteuse<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<HabillageDeLaPorteuse> {
    let ligne = sqlx::query!(
        r#"SELECT media.attached_image('org', 'organizations', o.id, 'logo') AS logo,
                  (SELECT t.label FROM reference.taxonomy_terms t
                    WHERE t.taxonomy_code = 'organization_type'
                      AND t.code = o.organization_type_code) AS type_label,
                  c.name AS "country_name?"
             FROM org.organizations o
             LEFT JOIN reference.countries c ON c.id = o.country_id
            WHERE o.id = $1"#,
        organization_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(match ligne {
        Some(l) => HabillageDeLaPorteuse {
            logo: l.logo,
            type_label: l.type_label,
            country_name: l.country_name,
        },
        None => HabillageDeLaPorteuse {
            logo: None,
            type_label: None,
            country_name: None,
        },
    })
}

/// Une organisation qui peut porter un dossier : vivante, ni absorbée, ni
/// archivée, ni refusée.
pub async fn organisation_peut_porter<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<bool> {
    let ok = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM org.organizations
                           WHERE id = $1 AND status IN ('candidate', 'active')) AS "ok!""#,
        organization_id
    )
    .fetch_one(executor)
    .await?;
    Ok(ok)
}

/// Une personne qui peut déposer : active, ni suspendue, ni effacée.
pub async fn personne_peut_deposer<'e>(
    executor: impl PgExecutor<'e>,
    person_id: Uuid,
) -> Result<bool> {
    let ok = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM identity.people
                           WHERE id = $1 AND status = 'active') AS "ok!""#,
        person_id
    )
    .fetch_one(executor)
    .await?;
    Ok(ok)
}
