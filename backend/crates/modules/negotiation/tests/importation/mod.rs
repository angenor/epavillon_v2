//! Ce que les tests de l'import partagent : une édition, son import allumé sur
//! un jeu archivé, un passage du travail, et la lecture de ce qu'il a écrit.

#![allow(dead_code)]

use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::testing::TestDb;
use negotiation::jobs::import::ImportOfficialSessions;
use serde_json::{json, Value};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

pub const PREMIER_JOUR: Date = time::macros::date!(2026 - 11 - 09);

pub struct Bac {
    pub base: TestDb,
    pub edition: Uuid,
}

impl Bac {
    /// Une édition à Antalya, son import allumé sur `cop30/lecture-1`, premier
    /// jour de l'archive posé sur le 9 novembre 2026.
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let edition = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO event.events
                   (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
               VALUES (2026, '{"fr":"COP31"}'::jsonb, 'cop31', '{"fr":"Description."}'::jsonb,
                       'online', 'Asia/Istanbul', '2026-11-09T00:00:00Z', '2026-11-20T00:00:00Z')
               RETURNING id"#,
        )
        .fetch_one(base.pool())
        .await
        .expect("insertion de l'édition");

        sqlx::query(
            "INSERT INTO negotiation.official_imports
                 (event_id, is_enabled, reader, archive_name, archive_first_day, official_programme_url)
             VALUES ($1, true, 'archive', 'cop30/lecture-1', $2, 'https://unfccc.int/cop31/schedule')",
        )
        .bind(edition)
        .bind(PREMIER_JOUR)
        .execute(base.pool())
        .await
        .expect("import de l'édition");

        Self { base, edition }
    }

    pub fn pool(&self) -> &PgPool {
        self.base.pool()
    }

    pub async fn jeu(&self, nom: &str) {
        sqlx::query(
            "UPDATE negotiation.official_imports SET archive_name = $2 WHERE event_id = $1",
        )
        .bind(self.edition)
        .bind(nom)
        .execute(self.pool())
        .await
        .expect("choix du jeu");
    }

    /// `affectation` est écrite par le test lui-même : « missed_threshold = 2 ».
    pub async fn regler(&self, affectation: &str) {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE negotiation.official_imports SET {affectation} WHERE event_id = $1"
        )))
        .bind(self.edition)
        .execute(self.pool())
        .await
        .expect("réglage de l'import");
    }

    pub async fn executer(&self, sql: &'static str) {
        sqlx::raw_sql(sql)
            .execute(self.pool())
            .await
            .expect("écriture du test");
    }

    /// Un passage du travail, comme le worker le ferait — sans clé de
    /// traduction.
    pub async fn lire(&self) -> ClaimedJob {
        self.passer(false, false).await
    }

    pub async fn lire_a_la_main(&self) -> ClaimedJob {
        self.passer(true, false).await
    }

    /// Un passage avec une clé de traduction : l'import pose la traduction.
    pub async fn lire_en_traduisant(&self) -> ClaimedJob {
        self.passer(false, true).await
    }

    async fn passer(&self, manuel: bool, traduire: bool) -> ClaimedJob {
        let travail = ClaimedJob {
            id: Uuid::now_v7(),
            queue: "default".into(),
            task: negotiation::jobs::import::IMPORT_OFFICIAL_SESSIONS.into(),
            payload: json!({ "event_id": self.edition, "manuel": manuel }),
            attempts: 1,
            max_attempts: 5,
        };
        ImportOfficialSessions::new(self.base.db(), traduire)
            .run(&travail)
            .await
            .expect("le travail d'import réussit toujours côté file");
        travail
    }

    pub async fn nombre(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar::<_, i64>(sql)
            .bind(self.edition)
            .fetch_one(self.pool())
            .await
            .expect("compte")
    }

    pub async fn sessions(&self) -> i64 {
        self.nombre("SELECT count(*) FROM negotiation.meetings WHERE event_id = $1")
            .await
    }

    pub async fn sert(&self) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT negotiation.import_is_serving($1)")
            .bind(self.edition)
            .fetch_one(self.pool())
            .await
            .expect("règle de coupure")
    }

    pub async fn session(&self, cle: &str) -> Option<Session> {
        sqlx::query_as::<_, Session>(
            r#"SELECT m.id, m.status::text AS status, m.cancellation_reason, m.cancelled_at,
                      m.start_at, m.end_at, m.venue_label, m.title_original, m.title::jsonb AS title,
                      m.source_url::text AS source_url, m.first_read_at, m.last_read_at,
                      m.absent_reads, m.is_open_access, m.kind::text AS kind,
                      m.is_ifdd_organized, m.timezone::text AS timezone,
                      t.code AS type_code, g.code AS group_code, a.code AS point_code,
                      a.title AS point_title
                 FROM negotiation.meetings m
                 LEFT JOIN reference.taxonomy_terms t ON t.id = m.meeting_type_term_id
                 LEFT JOIN reference.taxonomy_terms g ON g.id = m.group_term_id
                 LEFT JOIN negotiation.agenda_items a ON a.id = m.agenda_item_id
                WHERE m.event_id = $1 AND m.source_key = $2"#,
        )
        .bind(self.edition)
        .bind(cle)
        .fetch_optional(self.pool())
        .await
        .expect("lecture d'une session")
    }

    /// Les changements d'une session, dans l'ordre où ils ont été vus.
    pub async fn changements(&self, cle: &str) -> Vec<(String, Value, Value)> {
        sqlx::query_as::<_, (String, Option<Value>, Option<Value>)>(
            "SELECT c.field, c.old_value, c.new_value
               FROM negotiation.meeting_changes c
               JOIN negotiation.meetings m ON m.id = c.meeting_id
              WHERE m.event_id = $1 AND m.source_key = $2
              ORDER BY c.detected_at,
                       array_position(ARRAY['start','end','venue','title','type','access','agenda_item','status'], c.field)",
        )
        .bind(self.edition)
        .bind(cle)
        .fetch_all(self.pool())
        .await
        .expect("lecture des changements")
        .into_iter()
        .map(|(f, a, n)| (f, a.unwrap_or(Value::Null), n.unwrap_or(Value::Null)))
        .collect()
    }

    pub async fn etat(&self) -> EtatImport {
        sqlx::query_as::<_, EtatImport>(
            "SELECT missed_reads, failing_since, last_error, last_success_at, last_change_count
               FROM negotiation.official_imports WHERE event_id = $1",
        )
        .bind(self.edition)
        .fetch_one(self.pool())
        .await
        .expect("état de l'import")
    }

    /// Le journal, le plus récent d'abord.
    pub async fn journal(&self) -> Vec<(String, Option<i32>, Option<i32>, Option<String>)> {
        sqlx::query_as(
            "SELECT r.outcome, r.session_count, r.change_count, r.error
               FROM negotiation.import_runs r
               JOIN negotiation.official_imports i ON i.id = r.import_id
              WHERE i.event_id = $1
              ORDER BY r.started_at DESC, r.id DESC",
        )
        .bind(self.edition)
        .fetch_all(self.pool())
        .await
        .expect("journal")
    }

    /// Les lectures de la chaîne en file.
    pub async fn suivantes(&self) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT idempotency_key FROM platform.jobs
              WHERE task = 'negotiation.import_official_sessions' AND status = 'queued'
                AND starts_with(idempotency_key, 'import:' || $1::text || ':')
              ORDER BY idempotency_key",
        )
        .bind(self.edition)
        .fetch_all(self.pool())
        .await
        .expect("travaux en file")
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Session {
    pub id: Uuid,
    pub status: String,
    pub cancellation_reason: Option<String>,
    pub cancelled_at: Option<OffsetDateTime>,
    pub start_at: OffsetDateTime,
    pub end_at: Option<OffsetDateTime>,
    pub venue_label: Option<String>,
    pub title_original: String,
    pub title: Value,
    pub source_url: String,
    pub first_read_at: OffsetDateTime,
    pub last_read_at: OffsetDateTime,
    pub absent_reads: i16,
    pub is_open_access: Option<bool>,
    pub kind: String,
    pub is_ifdd_organized: bool,
    pub timezone: String,
    pub type_code: Option<String>,
    pub group_code: Option<String>,
    pub point_code: Option<String>,
    pub point_title: Option<String>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct EtatImport {
    pub missed_reads: i16,
    pub failing_since: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub last_success_at: Option<OffsetDateTime>,
    pub last_change_count: Option<i32>,
}
