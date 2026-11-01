//! Ce qui a changé dans le programme publié d'une édition (01/11), lu dans
//! `platform.audit_log` : la fenêtre « Ce qui a changé » de `/programmations`.
//!
//! **Même espace de noms que `cross/mod.rs`, mêmes règles** : lecture seule,
//! lecture n° 20 du tableau. Le journal porte la ligne complète de la séance
//! avant et après chaque écriture ; on n'en garde que ce que le public lisait
//! déjà, et jamais l'auteur (`actor_label` reste au back-office).

use kernel::error::Result;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Un changement visible du public. Une écriture peut en porter plusieurs
/// (titre et horaire ensemble) : une ligne chacun.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChangementDuProgramme {
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    /// `rescheduled`, `retitled`, `cancelled`, `postponed` ou `added`.
    pub kind: String,
    pub session_id: Uuid,
    pub slug: String,
    /// Titre courant, celui que porte aujourd'hui la page de l'activité.
    pub title: Value,
    /// Créneau juste après le changement.
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    /// Créneau juste avant ; nul pour une nouvelle activité.
    #[serde(with = "time::serde::rfc3339::option")]
    pub previous_starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub previous_ends_at: Option<OffsetDateTime>,
    /// Ancien et nouveau titre d'un `retitled` ; nuls sinon.
    pub previous_title: Option<Value>,
    pub new_title: Option<Value>,
    /// Un `rescheduled` qui change de jour civil dans le fuseau de la séance.
    pub day_changed: bool,
}

/// Les changements des quinze derniers jours, le plus récent d'abord.
///
/// Seulement les séances publiées aujourd'hui, et seulement ce qui leur est
/// arrivé **pendant** qu'elles l'étaient : un brouillon déplacé dix fois avant
/// publication n'a rien changé pour le public. La première publication du
/// programme n'est pas une suite d'« ajouts » : toutes ses séances partagent
/// l'instant de sa transaction, qu'on écarte. Une édition en brouillon,
/// annulée ou au programme non paru rend une liste vide.
pub async fn changements_du_programme<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
) -> Result<Vec<ChangementDuProgramme>> {
    let lignes = sqlx::query_as!(
        ChangementDuProgramme,
        r#"WITH journal AS (
               -- Par séance : l'index du journal est clé par entité, pas par édition.
               SELECT a.occurred_at, a.old_data AS o, a.new_data AS n,
                      s.id, s.slug::text AS slug, s.title, s.timezone::text AS tz
                 FROM event.events e
                 JOIN programme.sessions s ON s.event_id = e.id AND s.published_at IS NOT NULL
                 JOIN platform.audit_log a
                   ON a.entity_schema = 'programme' AND a.entity_table = 'sessions'
                  AND a.entity_id = s.id AND a.action IN ('insert', 'update')
                WHERE e.id = $1
                  AND e.status NOT IN ('draft', 'cancelled')
                  AND e.programme_published_at IS NOT NULL
           ),
           premiere AS (
               SELECT min(j.occurred_at) AS instant FROM journal j
                WHERE j.n ->> 'published_at' IS NOT NULL AND j.o ->> 'published_at' IS NULL
           ),
           changements AS (
               SELECT j.*, k.kind
                 FROM journal j
                CROSS JOIN premiere p
                CROSS JOIN LATERAL (
                    SELECT j.o ->> 'published_at' IS NOT NULL
                       AND j.n ->> 'published_at' IS NOT NULL AS publiee
                ) v
                CROSS JOIN LATERAL (VALUES
                    ('added', j.n ->> 'published_at' IS NOT NULL
                              AND j.o ->> 'published_at' IS NULL
                              AND j.occurred_at IS DISTINCT FROM p.instant),
                    ('rescheduled', v.publiee
                              AND (j.o -> 'starts_at' IS DISTINCT FROM j.n -> 'starts_at'
                                   OR j.o -> 'ends_at' IS DISTINCT FROM j.n -> 'ends_at')),
                    ('retitled', v.publiee AND j.o -> 'title' IS DISTINCT FROM j.n -> 'title'),
                    ('cancelled', v.publiee AND j.o ->> 'status' <> 'cancelled'
                              AND j.n ->> 'status' = 'cancelled'),
                    ('postponed', v.publiee AND j.o ->> 'status' <> 'postponed'
                              AND j.n ->> 'status' = 'postponed')
                ) AS k(kind, retenu)
                WHERE k.retenu AND j.occurred_at >= now() - interval '15 days'
           )
           SELECT c.occurred_at AS "occurred_at!", c.kind AS "kind!",
                  c.id AS "session_id!", c.slug AS "slug!", c.title AS "title!: Value",
                  (c.n ->> 'starts_at')::timestamptz AS "starts_at!",
                  (c.n ->> 'ends_at')::timestamptz AS "ends_at!",
                  CASE WHEN c.kind <> 'added' THEN (c.o ->> 'starts_at')::timestamptz END
                      AS previous_starts_at,
                  CASE WHEN c.kind <> 'added' THEN (c.o ->> 'ends_at')::timestamptz END
                      AS previous_ends_at,
                  CASE WHEN c.kind = 'retitled' THEN c.o -> 'title' END
                      AS "previous_title: Value",
                  CASE WHEN c.kind = 'retitled' THEN c.n -> 'title' END AS "new_title: Value",
                  (c.kind = 'rescheduled'
                   AND ((c.o ->> 'starts_at')::timestamptz AT TIME ZONE c.tz)::date
                       <> ((c.n ->> 'starts_at')::timestamptz AT TIME ZONE c.tz)::date)
                      AS "day_changed!"
             FROM changements c
            ORDER BY c.occurred_at DESC, c.kind
            LIMIT 300"#,
        event_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}
