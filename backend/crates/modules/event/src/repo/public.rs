//! **Ce que le public lit** — aucune session, aucun périmètre.
//!
//! Le critère de publicité n'est écrit nulle part ici : il vit dans
//! `event.v_public_editions`, donc dans le modèle (FR-084, écart n° 26). Une
//! édition est publique dès lors qu'elle n'est **ni un brouillon ni annulée** ;
//! une édition **annoncée** dont le programme n'est pas publié en fait partie.
//!
//! **Une requête, deux vues** (research.md § R16). `v_public_editions` porte
//! déjà la série, le pays, les **trois déclinaisons d'image** résolues par
//! `media.attached_image()`, l'état temporel et l'appel ; `v_edition_stats`
//! ajoute le volume du programme publié. Recomposer ces jointures ici, c'est
//! écrire une seconde fois ce que le modèle documente une fois.
//!
//! **La jointure est PAR LA GAUCHE, et c'est important** : `v_edition_stats` ne
//! porte que les éditions ayant au moins une séance publiée. Une jointure
//! stricte ferait disparaître de l'historique toute édition annoncée —
//! c'est-à-dire précisément celle sur laquelle on dépose un dossier. C'est la
//! leçon de B2, où une liste jointe par l'intérieur était vide sur base neuve.

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::ids::EventId;
use crate::domain::public::{
    PublicCall, PublicChannel, PublicDay, PublicEdition, PublicRoom, PublicSeries, PublicTrack,
    PublicVenue,
};

/// Les éditions publiques, **décroissantes sur la date de début** : la prochaine
/// COP est ce qu'on vient chercher.
pub async fn editions<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<PublicEdition>> {
    lire(executor, None).await
}

/// Une édition publique **par son adresse d'URL**.
///
/// `None` pour un brouillon, une annulée ou une adresse inconnue : **les trois
/// sont indiscernables**, et c'est voulu — sans quoi l'adresse d'une édition en
/// préparation se devinerait par la forme de la réponse.
pub async fn edition_par_slug<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
) -> Result<Option<PublicEdition>> {
    let mut lignes = lire(executor, Some(slug)).await?;
    Ok(lignes.pop())
}

async fn lire<'e>(executor: impl PgExecutor<'e>, slug: Option<&str>) -> Result<Vec<PublicEdition>> {
    let lignes = sqlx::query!(
        r#"SELECT v.id AS "id!", v.slug::text AS "slug!", v.title AS "title!", v.description AS "description!",
                  v.acronym, v.edition_label, v.edition_year AS "edition_year!",
                  v.status::text AS "status!",
                  v.participation_mode::text AS "participation_mode!",
                  v.timezone::text AS "timezone!",
                  v.starts_at AS "starts_at!", v.ends_at AS "ends_at!",
                  v.has_pavilion AS "has_pavilion!", v.programme_published_at, v.highlights,
                  v.series_id, v.series_kind::text AS "series_kind?",
                  v.series_name, v.series_slug::text AS "series_slug?",
                  v.country_id, v.country_code, v.country_name, v.city,
                  v.banner, v.cover, v.thumbnail,
                  v.temporal_state AS "temporal_state!",
                  v.call_id, v.call_status::text AS "call_status?",
                  v.call_is_open, v.call_deadline,
                  v.theme_codes AS "theme_codes!", v.themes AS "themes!",
                  e.address, e.latitude::float8 AS "latitude?", e.longitude::float8 AS "longitude?",
                  e.created_by, e.created_at AS "created_at!", e.updated_at AS "updated_at!",
                  COALESCE(st.published_session_count, 0) AS "published_session_count!",
                  COALESCE(st.streamed_session_count, 0)  AS "streamed_session_count!",
                  COALESCE(st.organization_count, 0)      AS "organization_count!",
                  st.programme_starts_at, st.programme_ends_at
             FROM event.v_public_editions v
             JOIN event.events e ON e.id = v.id
             LEFT JOIN programme.v_edition_stats st ON st.event_id = v.id
            WHERE $1::text IS NULL OR v.slug::text = $1
            ORDER BY v.starts_at DESC"#,
        slug
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicEdition {
            id: l.id,
            slug: l.slug,
            title: l.title,
            description: l.description,
            acronym: l.acronym,
            edition_label: l.edition_label,
            edition_year: l.edition_year,
            status: l.status,
            participation_mode: l.participation_mode,
            timezone: l.timezone,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            has_pavilion: l.has_pavilion,
            programme_published_at: l.programme_published_at,
            highlights: l.highlights,
            country_id: l.country_id,
            city: l.city,
            address: l.address,
            latitude: l.latitude,
            longitude: l.longitude,
            created_by: l.created_by,
            created_at: l.created_at,
            updated_at: l.updated_at,
            series_id: l.series_id,
            series_kind: l.series_kind,
            series_name: l.series_name,
            series_slug: l.series_slug,
            country_code: l.country_code,
            country_name: l.country_name,
            banner: l.banner,
            cover: l.cover,
            thumbnail: l.thumbnail,
            temporal_state: l.temporal_state,
            call_id: l.call_id,
            call_status: l.call_status,
            call_is_open: l.call_is_open,
            call_deadline: l.call_deadline,
            theme_codes: l.theme_codes,
            themes: l.themes,
            published_session_count: l.published_session_count,
            streamed_session_count: l.streamed_session_count,
            organization_count: l.organization_count,
            programme_starts_at: l.programme_starts_at,
            programme_ends_at: l.programme_ends_at,
        })
        .collect())
}

/// Les séries, avec leur **décompte d'éditions**.
///
/// **Toutes les séries**, actives ou non : la frise d'historique montre aussi
/// les cycles arrêtés, et `is_active` dit à l'écran comment les présenter. Le
/// décompte est joint par la gauche — une série sans édition reste visible, à
/// zéro.
pub async fn series<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<PublicSeries>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.code, s.kind::text AS "kind!", s.name, s.description,
                  s.slug::text AS "slug!", s.track_code, s.organizer_organization_id,
                  s.is_active, s.created_at, s.updated_at,
                  count(e.id) AS "edition_count!"
             FROM event.event_series s
             LEFT JOIN event.events e ON e.series_id = s.id
            GROUP BY s.id
            ORDER BY s.kind, s.name->>'fr'"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicSeries {
            id: l.id,
            code: l.code,
            kind: l.kind,
            name: l.name,
            description: l.description,
            slug: l.slug,
            track_code: l.track_code,
            organizer_organization_id: l.organizer_organization_id,
            is_active: l.is_active,
            created_at: l.created_at,
            updated_at: l.updated_at,
            edition_count: l.edition_count,
        })
        .collect())
}

/// Le calendrier d'une édition.
pub async fn journees<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PublicDay>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id, day_date, title, slug::text AS "slug?", description,
                  is_featured, color_hex, sort_order, created_at, updated_at
             FROM event.event_days
            WHERE event_id = $1
            ORDER BY day_date"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicDay {
            id: l.id,
            event_id: l.event_id,
            day_date: l.day_date,
            title: l.title,
            slug: l.slug,
            description: l.description,
            is_featured: l.is_featured,
            color_hex: l.color_hex,
            sort_order: l.sort_order,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// Les fils **publiés seulement**. Un fil dont la page n'est pas ouverte
/// n'existe pas pour le public : le filtre est `published_at IS NOT NULL`, la
/// colonne même que le modèle indexe pour cet usage.
pub async fn fils_publies<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PublicTrack>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id, code, slug::text AS "slug!", kind::text AS "kind!",
                  title, subtitle, description, starts_on, ends_on, color_hex,
                  curated_by, published_at, sort_order, created_at, updated_at
             FROM event.programme_tracks
            WHERE event_id = $1 AND published_at IS NOT NULL
            ORDER BY sort_order, starts_on NULLS LAST, code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicTrack {
            id: l.id,
            event_id: l.event_id,
            code: l.code,
            slug: l.slug,
            kind: l.kind,
            title: l.title,
            subtitle: l.subtitle,
            description: l.description,
            starts_on: l.starts_on,
            ends_on: l.ends_on,
            color_hex: l.color_hex,
            curated_by: l.curated_by,
            published_at: l.published_at,
            sort_order: l.sort_order,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// Les lieux d'une édition.
pub async fn lieux<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PublicVenue>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id, name, kind, address, map_url::text AS "map_url?", created_at
             FROM event.venues
            WHERE event_id = $1
            ORDER BY name->>'fr', id"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicVenue {
            id: l.id,
            event_id: l.event_id,
            name: l.name,
            kind: l.kind,
            address: l.address,
            map_url: l.map_url,
            created_at: l.created_at,
        })
        .collect())
}

/// **Les salles de tous les lieux de l'édition**, comme le front les compose :
/// une salle ne porte pas l'édition, elle la tient de son lieu.
pub async fn salles<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PublicRoom>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.venue_id, r.name, r.code, r.capacity, r.is_virtual,
                  r.has_streaming, r.equipment, r.sort_order, r.created_at
             FROM event.rooms r
             JOIN event.venues v ON v.id = r.venue_id
            WHERE v.event_id = $1
            ORDER BY r.sort_order, r.code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicRoom {
            id: l.id,
            venue_id: l.venue_id,
            name: l.name,
            code: l.code,
            capacity: l.capacity,
            is_virtual: l.is_virtual,
            has_streaming: l.has_streaming,
            equipment: l.equipment,
            sort_order: l.sort_order,
            created_at: l.created_at,
        })
        .collect())
}

/// Les canaux de l'édition **et ceux de la plateforme**, comme le front les
/// compose déjà. Un canal général sert les diffusions dont l'événement n'a pas
/// le sien : le taire ferait croire qu'aucun canal n'existe.
pub async fn canaux<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PublicChannel>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id, code, name, provider, channel_ref, locale,
                  is_default, is_active, created_at, updated_at
             FROM event.broadcast_channels
            WHERE event_id = $1 OR event_id IS NULL
            ORDER BY (event_id IS NULL), is_default DESC, code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicChannel {
            id: l.id,
            event_id: l.event_id,
            code: l.code,
            name: l.name,
            provider: l.provider,
            channel_ref: l.channel_ref,
            locale: l.locale,
            is_default: l.is_default,
            is_active: l.is_active,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// L'appel **non annulé** d'une édition. Zéro ou un, jamais un tableau.
pub async fn appel<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Option<PublicCall>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, code, title, description, status::text AS "status!",
                  opens_at, closes_at, extended_until, results_expected_at,
                  max_proposals_per_organization, requires_verified_organization,
                  min_speakers, max_speakers,
                  default_duration_minutes, min_duration_minutes, max_duration_minutes,
                  daily_start_time::text AS "daily_start_time!",
                  daily_end_time::text   AS "daily_end_time!",
                  allowed_formats::text[] AS "allowed_formats!",
                  required_reviews, blind_review,
                  guidelines_url::text AS "guidelines_url?",
                  created_by, created_at, updated_at
             FROM event.calls_for_proposals
            WHERE event_id = $1 AND status <> 'cancelled'"#,
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| PublicCall {
        id: l.id,
        event_id: l.event_id,
        code: l.code,
        title: l.title,
        description: l.description,
        status: l.status,
        opens_at: l.opens_at,
        closes_at: l.closes_at,
        extended_until: l.extended_until,
        results_expected_at: l.results_expected_at,
        max_proposals_per_organization: l.max_proposals_per_organization,
        requires_verified_organization: l.requires_verified_organization,
        min_speakers: l.min_speakers,
        max_speakers: l.max_speakers,
        default_duration_minutes: l.default_duration_minutes,
        min_duration_minutes: l.min_duration_minutes,
        max_duration_minutes: l.max_duration_minutes,
        daily_start_time: l.daily_start_time,
        daily_end_time: l.daily_end_time,
        allowed_formats: l.allowed_formats,
        required_reviews: l.required_reviews,
        blind_review: l.blind_review,
        guidelines_url: l.guidelines_url,
        created_by: l.created_by,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}
