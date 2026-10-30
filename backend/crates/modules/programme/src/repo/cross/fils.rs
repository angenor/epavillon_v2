//! Ce que la page publique d'un fil lit dans `event`, `org`, `reference` et
//! `media` (30/10) : le fil publié et son édition, son lieu, ses parties, ses
//! moments, ses partenaires, ses médias, et les autres fils publiés.
//!
//! **Même espace de noms que `cross/mod.rs`, mêmes règles** : la question porte
//! sur la composition publiée de ce module (« que montre la page de cette
//! journée ? »), et **aucune ligne n'écrit**. Lecture n° 18 du tableau.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgExecutor;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::track_page::{PublicTrackPartner, PublicTrackSummary};

/// Le fil publié, son édition et ses médias, en une ligne.
pub struct FilPublie {
    pub event_id: Uuid,
    pub event_slug: String,
    pub event_title: Value,
    pub event_acronym: Option<String>,
    pub timezone: String,
    pub event_starts_at: OffsetDateTime,
    pub event_ends_at: OffsetDateTime,
    pub city: Option<String>,
    pub country_name: Option<Value>,

    pub id: Uuid,
    pub slug: String,
    pub code: String,
    pub kind: String,
    pub title: Value,
    pub subtitle: Option<Value>,
    pub description: Option<Value>,
    pub tagline: Option<Value>,
    pub starts_on: Option<Date>,
    pub ends_on: Option<Date>,
    pub color_hex: Option<String>,
    pub themes: Value,
    pub target_audiences: Value,
    pub support_note: Option<Value>,
    pub feature_kicker: Option<Value>,
    pub feature_title: Option<Value>,
    pub feature_body: Option<Value>,
    pub feature_link_label: Option<Value>,
    pub feature_link_url: Option<String>,

    pub cover: Option<Value>,
    pub feature_visual: Option<Value>,
    pub documents: Value,

    /// Le lieu résolu : celui du fil, à défaut le pavillon de l'édition.
    pub venue_id: Option<Uuid>,
    pub venue_name: Option<Value>,
    pub venue_kind: Option<String>,
    pub venue_address: Option<String>,
    pub venue_map_url: Option<String>,
    pub venue_area: Option<Value>,
    pub venue_access_note: Option<Value>,
    pub venue_latitude: Option<f64>,
    pub venue_longitude: Option<f64>,
    /// Vrai quand le fil désigne un lieu qui n'est pas un pavillon.
    pub is_off_site: bool,
}

/// Le fil **publié** d'une édition **publique**, par leurs deux adresses.
/// `None` pour un fil non publié, une édition en brouillon ou annulée, une
/// adresse inconnue : indiscernables.
pub async fn fil_publie<'e>(
    executor: impl PgExecutor<'e>,
    event_slug: &str,
    track_slug: &str,
) -> Result<Option<FilPublie>> {
    let ligne = sqlx::query_as!(
        FilPublie,
        r#"WITH f AS (
               SELECT t.*, e.slug::text AS e_slug, e.title AS e_title, e.acronym AS e_acronym,
                      e.timezone::text AS e_timezone, e.starts_at AS e_starts_at,
                      e.ends_at AS e_ends_at, e.city AS e_city, c.name AS e_country,
                      COALESCE(t.venue_id, (
                          SELECT v.id FROM event.venues v
                           WHERE v.event_id = e.id AND v.kind = 'pavilion'
                           ORDER BY v.created_at, v.id LIMIT 1)) AS lieu_id
                 FROM event.programme_tracks t
                 JOIN event.events e ON e.id = t.event_id
                 LEFT JOIN reference.countries c ON c.id = e.country_id
                WHERE e.slug = $1 AND t.slug = $2
                  AND t.published_at IS NOT NULL
                  AND e.status NOT IN ('draft', 'cancelled')
           )
           SELECT f.event_id AS "event_id!", f.e_slug AS "event_slug!",
                  f.e_title AS "event_title!: Value", f.e_acronym AS event_acronym,
                  f.e_timezone AS "timezone!", f.e_starts_at AS "event_starts_at!",
                  f.e_ends_at AS "event_ends_at!", f.e_city AS city,
                  f.e_country AS "country_name: Value",
                  f.id AS "id!", f.slug::text AS "slug!", f.code AS "code!",
                  f.kind::text AS "kind!", f.title AS "title!: Value",
                  f.subtitle AS "subtitle: Value", f.description AS "description: Value",
                  f.tagline AS "tagline: Value", f.starts_on, f.ends_on, f.color_hex,
                  reference.term_badges('event', 'programme_tracks', f.id, 'activity_theme')
                      AS "themes!: Value",
                  to_jsonb(f.target_audiences) AS "target_audiences!: Value",
                  f.support_note AS "support_note: Value",
                  f.feature_kicker AS "feature_kicker: Value",
                  f.feature_title AS "feature_title: Value",
                  f.feature_body AS "feature_body: Value",
                  f.feature_link_label AS "feature_link_label: Value",
                  f.feature_link_url,
                  media.attached_image('event', 'programme_tracks', f.id, 'cover') AS "cover: Value",
                  media.attached_image('event', 'programme_tracks', f.id, 'thumbnail')
                      AS "feature_visual: Value",
                  media.attached_documents('event', 'programme_tracks', f.id, 'document')
                      AS "documents!: Value",
                  v.id AS "venue_id?", v.name AS "venue_name?: Value", v.kind AS "venue_kind?",
                  v.address AS "venue_address?", v.map_url::text AS "venue_map_url?",
                  v.area AS "venue_area?: Value", v.access_note AS "venue_access_note?: Value",
                  v.latitude::float8 AS "venue_latitude?", v.longitude::float8 AS "venue_longitude?",
                  (f.venue_id IS NOT NULL AND v.kind IS DISTINCT FROM 'pavilion') AS "is_off_site!"
             FROM f
             LEFT JOIN event.venues v ON v.id = f.lieu_id"#,
        event_slug,
        track_slug
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne)
}

/// Une partie du déroulé, telle que la page la lit.
pub struct PartieDuFil {
    pub id: Uuid,
    pub label: Option<Value>,
    pub title: Option<Value>,
    pub summary: Option<Value>,
}

pub async fn parties_du_fil<'e>(
    executor: impl PgExecutor<'e>,
    track_id: Uuid,
) -> Result<Vec<PartieDuFil>> {
    let lignes = sqlx::query_as!(
        PartieDuFil,
        r#"SELECT id, label AS "label: Value", title AS "title: Value",
                  summary AS "summary: Value"
             FROM event.programme_track_parts
            WHERE track_id = $1
            ORDER BY sort_order, created_at"#,
        track_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Un moment sans activité, tel que la page le lit.
pub struct MomentDuFil {
    pub id: Uuid,
    pub part_id: Option<Uuid>,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub label: Value,
    pub description: Option<Value>,
}

pub async fn moments_du_fil<'e>(
    executor: impl PgExecutor<'e>,
    track_id: Uuid,
) -> Result<Vec<MomentDuFil>> {
    let lignes = sqlx::query_as!(
        MomentDuFil,
        r#"SELECT id, part_id, starts_at, ends_at, label AS "label!: Value",
                  description AS "description: Value"
             FROM event.programme_track_moments
            WHERE track_id = $1
            ORDER BY starts_at, created_at"#,
        track_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les partenaires du fil, dans leur ordre, résolus pour l'affichage.
pub async fn partenaires_du_fil<'e>(
    executor: impl PgExecutor<'e>,
    track_id: Uuid,
) -> Result<Vec<PublicTrackPartner>> {
    let lignes = sqlx::query_as!(
        PublicTrackPartner,
        r#"SELECT p.organization_id, o.legal_name AS name, o.acronym,
                  media.attached_image('org', 'organizations', o.id, 'logo') AS "logo: Value",
                  t.code AS role_code, t.label AS "role_label!: Value"
             FROM event.programme_track_partners p
             JOIN org.organizations o ON o.id = p.organization_id
             JOIN reference.taxonomy_terms t ON t.id = p.role_term_id
            WHERE p.track_id = $1
            ORDER BY p.sort_order, o.legal_name"#,
        track_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les fils publiés de l'édition, chacun avec l'étendue de son déroulé publié
/// (activités publiées et moments) et son lieu, pour relier les pages entre
/// elles. `courant` marque la page lue.
pub async fn fils_publies_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
    courant: Uuid,
) -> Result<Vec<PublicTrackSummary>> {
    let lignes = sqlx::query!(
        r#"WITH bornes AS (
               SELECT st.track_id, s.starts_at AS debut, s.ends_at AS fin
                 FROM programme.session_tracks st
                 JOIN programme.sessions s ON s.id = st.session_id
                WHERE s.event_id = $1 AND s.published_at IS NOT NULL
               UNION ALL
               SELECT m.track_id, m.starts_at, COALESCE(m.ends_at, m.starts_at)
                 FROM event.programme_track_moments m
                 JOIN event.programme_tracks t ON t.id = m.track_id
                WHERE t.event_id = $1
           )
           SELECT t.id, t.slug::text AS "slug!", t.kind::text AS "kind!",
                  t.title AS "title!: Value", t.color_hex, t.starts_on, t.ends_on,
                  (SELECT min(b.debut) FROM bornes b WHERE b.track_id = t.id) AS starts_at,
                  (SELECT max(b.fin)   FROM bornes b WHERE b.track_id = t.id) AS ends_at,
                  COALESCE(v.name, (
                      SELECT p.name FROM event.venues p
                       WHERE p.event_id = t.event_id AND p.kind = 'pavilion'
                       ORDER BY p.created_at, p.id LIMIT 1)) AS "venue_name: Value",
                  (t.venue_id IS NOT NULL AND v.kind IS DISTINCT FROM 'pavilion') AS "is_off_site!"
             FROM event.programme_tracks t
             LEFT JOIN event.venues v ON v.id = t.venue_id
            WHERE t.event_id = $1 AND t.published_at IS NOT NULL
            ORDER BY t.starts_on NULLS LAST, t.sort_order, t.code"#,
        event_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicTrackSummary {
            is_current: l.id == courant,
            slug: l.slug,
            kind: l.kind,
            title: l.title,
            color_hex: l.color_hex,
            starts_on: l.starts_on,
            ends_on: l.ends_on,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            venue_name: l.venue_name,
            is_off_site: l.is_off_site,
        })
        .collect())
}
