//! Le back-office de l'import : lire l'état, poser le réglage, rattacher un
//! point de l'ordre du jour. La règle de coupure est lue par
//! `negotiation.import_is_serving()`, jamais recopiée.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::admin_import::{
    AgendaItemAdmin, EditionImport, ImportRun, UpdateOfficialImportPayload,
};

pub struct Edition {
    pub id: Uuid,
    pub edition: EditionImport,
}

pub async fn edition(conn: &mut PgConnection, slug: &str) -> Result<Option<Edition>> {
    let ligne = sqlx::query!(
        r#"SELECT id, slug::text AS "slug!", title::jsonb AS "title!: Value",
                  timezone::text AS "timezone!"
             FROM event.events WHERE slug::text = $1"#,
        slug
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Edition {
        id: l.id,
        edition: EditionImport {
            slug: l.slug,
            name: l.title,
            timezone: l.timezone,
        },
    }))
}

pub struct Etat {
    pub import_id: Uuid,
    pub enabled: bool,
    pub reader: String,
    pub archive_name: Option<String>,
    pub archive_first_day: Option<Date>,
    pub live_url: Option<String>,
    pub time_correction_minutes: i16,
    pub official_programme_url: String,
    pub interval_seconds: i32,
    pub missed_threshold: i16,
    pub missed_reads: i16,
    pub serving: bool,
    pub last_success_at: Option<OffsetDateTime>,
    pub last_attempt_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub failing_since: Option<OffsetDateTime>,
    pub last_change_count: Option<i32>,
}

pub async fn etat(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<Etat>> {
    let ligne = sqlx::query!(
        r#"SELECT id, is_enabled, reader, archive_name, archive_first_day,
                  live_url::text AS live_url, time_correction_minutes,
                  official_programme_url::text AS "official_programme_url!",
                  interval_seconds, missed_threshold, missed_reads,
                  negotiation.import_is_serving(event_id) AS "serving!",
                  last_success_at, last_attempt_at, last_error, failing_since, last_change_count
             FROM negotiation.official_imports WHERE event_id = $1"#,
        event_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Etat {
        import_id: l.id,
        enabled: l.is_enabled,
        reader: l.reader,
        archive_name: l.archive_name,
        archive_first_day: l.archive_first_day,
        live_url: l.live_url,
        time_correction_minutes: l.time_correction_minutes,
        official_programme_url: l.official_programme_url,
        interval_seconds: l.interval_seconds,
        missed_threshold: l.missed_threshold,
        missed_reads: l.missed_reads,
        serving: l.serving,
        last_success_at: l.last_success_at,
        last_attempt_at: l.last_attempt_at,
        last_error: l.last_error,
        failing_since: l.failing_since,
        last_change_count: l.last_change_count,
    }))
}

/// Les sessions importées, et les points que personne n'a encore rattachés.
pub async fn comptes(conn: &mut PgConnection, event_id: Uuid) -> Result<(i64, i64)> {
    let l = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM negotiation.meetings
                    WHERE event_id = $1 AND source_key IS NOT NULL) AS "sessions!",
                  (SELECT count(*) FROM negotiation.agenda_items
                    WHERE event_id = $1 AND theme_term_id IS NULL) AS "sans_theme!""#,
        event_id
    )
    .fetch_one(conn)
    .await?;
    Ok((l.sessions, l.sans_theme))
}

pub async fn journal(conn: &mut PgConnection, import_id: Uuid) -> Result<Vec<ImportRun>> {
    let lignes = sqlx::query!(
        "SELECT started_at, outcome, error, session_count, change_count, is_manual
           FROM negotiation.import_runs
          WHERE import_id = $1
          ORDER BY started_at DESC, id DESC
          LIMIT 20",
        import_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| ImportRun {
            started_at: l.started_at,
            outcome: l.outcome,
            error: l.error,
            session_count: l.session_count,
            change_count: l.change_count,
            manual: l.is_manual,
        })
        .collect())
}

/// Crée la ligne si elle manque. Les compteurs de santé ne bougent pas : ils
/// appartiennent aux lectures.
pub async fn regler(
    conn: &mut PgConnection,
    event_id: Uuid,
    r: &UpdateOfficialImportPayload,
    acteur: Uuid,
) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO negotiation.official_imports
               (event_id, is_enabled, reader, archive_name, archive_first_day, live_url,
                time_correction_minutes, official_programme_url, interval_seconds,
                missed_threshold, updated_by)
           VALUES ($1, $2, $3, $4, $5, $6::text::platform.url, $7, $8::text::platform.url,
                   $9, $10, $11)
           ON CONFLICT (event_id) DO UPDATE
              SET is_enabled = EXCLUDED.is_enabled, reader = EXCLUDED.reader,
                  archive_name = EXCLUDED.archive_name,
                  archive_first_day = EXCLUDED.archive_first_day,
                  live_url = EXCLUDED.live_url,
                  time_correction_minutes = EXCLUDED.time_correction_minutes,
                  official_programme_url = EXCLUDED.official_programme_url,
                  interval_seconds = EXCLUDED.interval_seconds,
                  missed_threshold = EXCLUDED.missed_threshold,
                  updated_by = EXCLUDED.updated_by"#,
        event_id,
        r.enabled,
        r.reader,
        r.archive_name,
        r.archive_first_day,
        r.live_url,
        r.time_correction_minutes,
        r.official_programme_url,
        r.interval_seconds,
        r.missed_threshold,
        acteur
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Sans thématique d'abord, puis par code.
pub async fn points(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<AgendaItemAdmin>> {
    let lignes = sqlx::query!(
        r#"SELECT a.id, a.code, a.title, t.code AS "theme?", a.theme_set_at,
                  (SELECT count(*) FROM negotiation.meetings m
                    WHERE m.agenda_item_id = a.id) AS "sessions!"
             FROM negotiation.agenda_items a
             LEFT JOIN reference.taxonomy_terms t ON t.id = a.theme_term_id
            WHERE a.event_id = $1
            ORDER BY a.theme_term_id IS NOT NULL, a.code"#,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AgendaItemAdmin {
            id: l.id,
            code: l.code,
            title: l.title,
            theme: l.theme,
            session_count: l.sessions,
            theme_set_at: l.theme_set_at,
        })
        .collect())
}

pub async fn point(conn: &mut PgConnection, id: Uuid) -> Result<Option<AgendaItemAdmin>> {
    let ligne = sqlx::query!(
        r#"SELECT a.id AS "id!", a.code AS "code!", a.title AS "title!", t.code AS "theme?",
                  a.theme_set_at, (SELECT count(*) FROM negotiation.meetings m
                                    WHERE m.agenda_item_id = a.id) AS "sessions!"
             FROM negotiation.agenda_items a
             LEFT JOIN reference.taxonomy_terms t ON t.id = a.theme_term_id
            WHERE a.id = $1"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| AgendaItemAdmin {
        id: l.id,
        code: l.code,
        title: l.title,
        theme: l.theme,
        session_count: l.sessions,
        theme_set_at: l.theme_set_at,
    }))
}

/// Une thématique proposée, par son code.
pub async fn theme_actif(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_theme' AND code = $1 AND is_active",
        code
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

/// Faux : le point n'existe pas.
pub async fn rattacher(
    conn: &mut PgConnection,
    id: Uuid,
    theme: Option<Uuid>,
    acteur: Uuid,
) -> Result<bool> {
    let touche = sqlx::query!(
        "UPDATE negotiation.agenda_items
            SET theme_term_id = $2,
                theme_set_by = CASE WHEN $2::uuid IS NULL THEN NULL ELSE $3::uuid END,
                theme_set_at = CASE WHEN $2::uuid IS NULL THEN NULL ELSE now() END
          WHERE id = $1",
        id,
        theme,
        acteur
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(touche == 1)
}
