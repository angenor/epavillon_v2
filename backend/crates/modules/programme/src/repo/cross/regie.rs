//! La régie du jour (02/11) : ce que l'équipe voit d'une journée. Lecture
//! n° 21 du tableau : `event.events`, `event.event_days`, `event.rooms`,
//! `org.organizations`, `media`, `identity.people` (`display_name` seul),
//! `live.meetings` et `live.meeting_room_counts`, `live.incidents`,
//! `reference.taxonomy_terms`, et `platform.jobs` pour savoir si les courriels
//! d'une annulation sont partis. Lecture seule, comme tout ce dossier.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgExecutor;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

pub struct EditionDeLaRegie {
    pub title: Value,
    pub acronym: Option<String>,
    pub timezone: String,
    pub city: Option<String>,
    pub today: Date,
    pub now: OffsetDateTime,
}

pub async fn edition_de_la_regie<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
) -> Result<Option<EditionDeLaRegie>> {
    let ligne = sqlx::query!(
        r#"SELECT e.title AS "title!: Value", e.acronym, e.timezone::text AS "timezone!", e.city,
                  (now() AT TIME ZONE e.timezone)::date AS "today!", now() AS "now!"
             FROM event.events e WHERE e.id = $1"#,
        event_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne.map(|l| EditionDeLaRegie {
        title: l.title,
        acronym: l.acronym,
        timezone: l.timezone,
        city: l.city,
        today: l.today,
        now: l.now,
    }))
}

/// Les jours du programme : ceux de l'édition, et ceux où une séance publiée
/// se tient, dans le fuseau de l'édition.
pub async fn jours_du_programme<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
) -> Result<Vec<Date>> {
    let jours = sqlx::query_scalar!(
        r#"SELECT DISTINCT j AS "j!" FROM (
               SELECT d.day_date AS j FROM event.event_days d WHERE d.event_id = $1
               UNION
               SELECT (s.starts_at AT TIME ZONE e.timezone)::date
                 FROM programme.sessions s JOIN event.events e ON e.id = s.event_id
                WHERE s.event_id = $1 AND s.published_at IS NOT NULL
           ) t ORDER BY 1"#,
        event_id
    )
    .fetch_all(executor)
    .await?;
    Ok(jours)
}

pub struct SeanceDeLaRegie {
    pub id: Uuid,
    pub proposal_id: Option<Uuid>,
    pub title: Value,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub status: String,
    pub is_published: bool,
    pub is_streamed: bool,
    pub is_joint: bool,
    pub room_name: Option<Value>,
    pub cover: Option<Value>,
    pub registrants: i64,
    pub presence: Option<bool>,
    pub presence_noted_at: Option<OffsetDateTime>,
    pub presence_noted_by: Option<String>,
    pub meeting_status: Option<String>,
    pub meeting_sync_status: Option<String>,
    pub meeting_ready: bool,
    pub meeting_present: Option<i64>,
    pub live_started_at: Option<OffsetDateTime>,
    pub live_ended_at: Option<OffsetDateTime>,
    pub cancelled_at: Option<OffsetDateTime>,
    pub reason_code: Option<String>,
    pub reason_label: Option<Value>,
    pub cancellation_note: Option<String>,
}

/// Les séances publiées d'un jour, dans l'ordre du créneau.
pub async fn seances_du_jour<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
    jour: Date,
) -> Result<Vec<SeanceDeLaRegie>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.proposal_id, s.title AS "title!: Value", s.starts_at, s.ends_at,
                  s.status::text AS "status!", (s.published_at IS NOT NULL) AS "is_published!",
                  s.is_streamed,
                  (s.proposal_id IS NULL AND EXISTS (
                       SELECT 1 FROM programme.session_proposals sp WHERE sp.session_id = s.id
                  )) AS "is_joint!",
                  r.name AS "room_name?: Value",
                  COALESCE(media.attached_image('programme', 'sessions', s.id, 'cover'),
                           media.attached_image('programme', 'proposals', COALESCE(s.proposal_id,
                               (SELECT sp.proposal_id FROM programme.session_proposals sp
                                 WHERE sp.session_id = s.id ORDER BY sp.sort_order LIMIT 1)), 'cover'))
                      AS "cover?: Value",
                  (SELECT count(*) FROM programme.registrations g
                    WHERE g.session_id = s.id AND g.status IN ('registered', 'attended')) AS "registrants!",
                  pr.is_present AS "presence?", pr.noted_at AS "presence_noted_at?",
                  pp.display_name AS "presence_noted_by?",
                  m.status AS "meeting_status?", m.sync_status AS "meeting_sync_status?",
                  COALESCE(m.ready, false) AS "meeting_ready!",
                  m.present AS "meeting_present?",
                  s.live_started_at, s.live_ended_at, s.cancelled_at,
                  s.cancellation_reason_code AS "reason_code?",
                  t.label AS "reason_label?: Value",
                  s.cancellation_note AS "cancellation_note?"
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
             LEFT JOIN event.rooms r ON r.id = s.room_id
             LEFT JOIN programme.session_presence pr ON pr.session_id = s.id
             LEFT JOIN identity.people pp ON pp.id = pr.noted_by
             LEFT JOIN reference.taxonomy_terms t
                    ON t.taxonomy_code = 'session_cancellation_reason'
                   AND t.code = s.cancellation_reason_code
             LEFT JOIN LATERAL (
                 SELECT mm.status::text AS status, mm.sync_status::text AS sync_status,
                        (mm.external_id IS NOT NULL AND mm.status IN ('scheduled', 'started')) AS ready,
                        CASE WHEN mm.status = 'started' THEN c.present_count END AS present
                   FROM live.meetings mm
                   LEFT JOIN live.meeting_room_counts c ON c.meeting_id = mm.id
                  -- Une activité de journée : la réunion de la journée (02/11).
                  WHERE (mm.session_id = s.id OR mm.track_id = programme.day_track_of(s.id))
                    AND mm.status NOT IN ('cancelled', 'failed')
                  ORDER BY mm.created_at DESC
                  LIMIT 1
             ) m ON true
            WHERE s.event_id = $1
              AND s.published_at IS NOT NULL
              AND (s.starts_at AT TIME ZONE e.timezone)::date = $2
            ORDER BY s.starts_at, s.id"#,
        event_id,
        jour
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| SeanceDeLaRegie {
            id: l.id,
            proposal_id: l.proposal_id,
            title: l.title,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            status: l.status,
            is_published: l.is_published,
            is_streamed: l.is_streamed,
            is_joint: l.is_joint,
            room_name: l.room_name,
            cover: l.cover,
            registrants: l.registrants,
            presence: l.presence,
            presence_noted_at: l.presence_noted_at,
            presence_noted_by: l.presence_noted_by,
            meeting_status: l.meeting_status,
            meeting_sync_status: l.meeting_sync_status,
            meeting_ready: l.meeting_ready,
            meeting_present: l.meeting_present,
            live_started_at: l.live_started_at,
            live_ended_at: l.live_ended_at,
            cancelled_at: l.cancelled_at,
            reason_code: l.reason_code,
            reason_label: l.reason_label,
            cancellation_note: l.cancellation_note,
        })
        .collect())
}

pub struct OrganisationDeSeance {
    pub session_id: Uuid,
    pub id: Uuid,
    pub name: String,
    pub acronym: Option<String>,
    pub logo: Option<Value>,
}

/// Les organisations des séances, porteuses d'abord.
pub async fn organisations_des_seances<'e>(
    executor: impl PgExecutor<'e>,
    seances: &[Uuid],
) -> Result<Vec<OrganisationDeSeance>> {
    let lignes = sqlx::query!(
        r#"SELECT so.session_id, o.id, o.legal_name AS name, o.acronym,
                  media.attached_image('org', 'organizations', o.id, 'logo') AS "logo?: Value"
             FROM programme.session_organizations so
             JOIN org.organizations o ON o.id = so.organization_id
            WHERE so.session_id = ANY($1)
            ORDER BY so.session_id, (so.role <> 'lead'), so.sort_order, o.legal_name"#,
        seances
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| OrganisationDeSeance {
            session_id: l.session_id,
            id: l.id,
            name: l.name,
            acronym: l.acronym,
            logo: l.logo,
        })
        .collect())
}

pub struct PieceDeSeance {
    pub session_id: Uuid,
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub title: Value,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub byte_size: Option<i64>,
    pub url: Option<String>,
    pub uploaded_at: OffsetDateTime,
}

/// Les pièces déposées avec le dossier de chaque séance, tous les dossiers
/// d'une séance commune.
pub async fn pieces_des_seances<'e>(
    executor: impl PgExecutor<'e>,
    seances: &[Uuid],
) -> Result<Vec<PieceDeSeance>> {
    let lignes = sqlx::query!(
        r#"SELECT x.session_id AS "session_id!", d.id, d.proposal_id, d.title AS "title!: Value",
                  a.original_filename AS "filename?", a.mime_type AS "mime_type?",
                  a.byte_size AS "byte_size?",
                  CASE WHEN a.status = 'ready' AND a.deleted_at IS NULL
                       THEN media.object_url(a.bucket, a.object_key) END AS "url?",
                  d.uploaded_at
             FROM (
                 SELECT s.id AS session_id, s.proposal_id, 0 AS rang
                   FROM programme.sessions s
                  WHERE s.id = ANY($1) AND s.proposal_id IS NOT NULL
                 UNION ALL
                 SELECT sp.session_id, sp.proposal_id, sp.sort_order
                   FROM programme.session_proposals sp
                  WHERE sp.session_id = ANY($1)
             ) x
             JOIN programme.proposal_documents d ON d.proposal_id = x.proposal_id
             LEFT JOIN media.assets a ON a.id = d.asset_id
            ORDER BY x.session_id, x.rang, d.sort_order, d.uploaded_at"#,
        seances
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| PieceDeSeance {
            session_id: l.session_id,
            id: l.id,
            proposal_id: l.proposal_id,
            title: l.title,
            filename: l.filename,
            mime_type: l.mime_type,
            byte_size: l.byte_size,
            url: l.url,
            uploaded_at: l.uploaded_at,
        })
        .collect())
}

pub struct GesteDuJournal {
    pub id: Uuid,
    pub action_code: String,
    pub session_id: Uuid,
    pub session_title: Value,
    pub session_starts_at: OffsetDateTime,
    pub session_status: String,
    pub session_live_ended_at: Option<OffsetDateTime>,
    pub data: Value,
    pub acted_at: OffsetDateTime,
    pub acted_by: Option<Uuid>,
    pub actor_name: Option<String>,
    pub undone_at: Option<OffsetDateTime>,
    pub undone_by: Option<String>,
}

/// Les gestes de la régie posés ce jour-là, ou visant une séance de ce jour.
pub async fn gestes_du_jour<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
    jour: Date,
) -> Result<Vec<GesteDuJournal>> {
    let lignes = sqlx::query!(
        r#"SELECT a.id, a.action_code, a.session_id, s.title AS "session_title!: Value",
                  s.starts_at AS session_starts_at, s.status::text AS "session_status!",
                  s.live_ended_at AS session_live_ended_at, a.data, a.acted_at, a.acted_by,
                  p.display_name AS "actor_name?", a.undone_at, u.display_name AS "undone_by?"
             FROM programme.control_room_actions a
             JOIN programme.sessions s ON s.id = a.session_id
             JOIN event.events e ON e.id = a.event_id
             LEFT JOIN identity.people p ON p.id = a.acted_by
             LEFT JOIN identity.people u ON u.id = a.undone_by
            WHERE a.event_id = $1
              AND ((a.acted_at AT TIME ZONE e.timezone)::date = $2
                   OR (s.starts_at AT TIME ZONE e.timezone)::date = $2)
            ORDER BY a.acted_at DESC, a.id DESC
            LIMIT 200"#,
        event_id,
        jour
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| GesteDuJournal {
            id: l.id,
            action_code: l.action_code,
            session_id: l.session_id,
            session_title: l.session_title,
            session_starts_at: l.session_starts_at,
            session_status: l.session_status,
            session_live_ended_at: l.session_live_ended_at,
            data: l.data,
            acted_at: l.acted_at,
            acted_by: l.acted_by,
            actor_name: l.actor_name,
            undone_at: l.undone_at,
            undone_by: l.undone_by,
        })
        .collect())
}

pub struct MessagePublie {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub session_title: Option<Value>,
    pub session_starts_at: Option<OffsetDateTime>,
    pub message: Value,
    pub kind_code: String,
    pub published_at: OffsetDateTime,
    pub published_by: Option<Uuid>,
    pub publisher_name: Option<String>,
    pub unpublished_at: Option<OffsetDateTime>,
    pub unpublisher_name: Option<String>,
    pub still_shown: bool,
}

/// Les messages d'incident de l'édition publiés ce jour-là.
pub async fn messages_du_jour<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
    jour: Date,
) -> Result<Vec<MessagePublie>> {
    let lignes = sqlx::query!(
        r#"SELECT i.id, i.session_id, s.title AS "session_title?: Value",
                  s.starts_at AS "session_starts_at?", i.message AS "message!: Value",
                  i.incident_kind_code, i.published_at AS "published_at!", i.published_by,
                  p.display_name AS "publisher_name?", i.unpublished_at,
                  u.display_name AS "unpublisher_name?",
                  (i.unpublished_at IS NULL AND (i.display_until IS NULL OR i.display_until > now()))
                      AS "still_shown!"
             FROM live.incidents i
             JOIN event.events e ON e.id = $1
             LEFT JOIN programme.sessions s ON s.id = i.session_id
             LEFT JOIN event.event_days d ON d.id = i.event_day_id
             LEFT JOIN identity.people p ON p.id = i.published_by
             LEFT JOIN identity.people u ON u.id = i.unpublished_by
            WHERE i.published_at IS NOT NULL
              AND (i.event_id = $1 OR s.event_id = $1 OR d.event_id = $1)
              AND (i.published_at AT TIME ZONE e.timezone)::date = $2
            ORDER BY i.published_at DESC
            LIMIT 100"#,
        event_id,
        jour
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| MessagePublie {
            id: l.id,
            session_id: l.session_id,
            session_title: l.session_title,
            session_starts_at: l.session_starts_at,
            message: l.message,
            kind_code: l.incident_kind_code,
            published_at: l.published_at,
            published_by: l.published_by,
            publisher_name: l.publisher_name,
            unpublished_at: l.unpublished_at,
            unpublisher_name: l.unpublisher_name,
            still_shown: l.still_shown,
        })
        .collect())
}

/// Ce que `engagement` a fait des courriels d'une annulation.
pub struct EnvoiDesCourriels {
    pub action_id: Uuid,
    /// Le travail d'envoi a abouti (il a pu ne rien envoyer si le geste a été
    /// défait avant).
    pub done_at: Option<OffsetDateTime>,
    pub sent: i64,
    pub last_sent_at: Option<OffsetDateTime>,
}

pub async fn envois_des_annulations<'e>(
    executor: impl PgExecutor<'e>,
    actions: &[Uuid],
) -> Result<Vec<EnvoiDesCourriels>> {
    let lignes = sqlx::query!(
        r#"SELECT a AS "action_id!",
                  (SELECT j.completed_at FROM platform.jobs j
                    WHERE j.task = $2 AND j.idempotency_key = 'cancelled:' || a::text
                      AND j.status = 'succeeded'
                    ORDER BY j.completed_at DESC LIMIT 1) AS done_at,
                  (SELECT count(*) FROM platform.jobs j
                    WHERE j.task = $3 AND j.idempotency_key LIKE 'regie:' || a::text || ':%'
                      AND j.status <> 'cancelled') AS "sent!",
                  (SELECT max(j.completed_at) FROM platform.jobs j
                    WHERE j.task = $3 AND j.idempotency_key LIKE 'regie:' || a::text || ':%'
                      AND j.status = 'succeeded') AS last_sent_at
             FROM unnest($1::uuid[]) AS a"#,
        actions,
        contracts::programme::REGIE_NOTICE_TASK,
        contracts::programme::NOTIFICATION_EMAIL_TASK
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| EnvoiDesCourriels {
            action_id: l.action_id,
            done_at: l.done_at,
            sent: l.sent,
            last_sent_at: l.last_sent_at,
        })
        .collect())
}

/// Un direct en cours, toutes éditions confondues (règle n° 4).
pub struct DirectEnCours {
    pub session_id: Uuid,
    pub event_id: Uuid,
    pub title: Value,
}

pub async fn direct_en_cours<'e>(
    executor: impl PgExecutor<'e>,
    sauf: Option<Uuid>,
) -> Result<Option<DirectEnCours>> {
    let ligne = sqlx::query!(
        r#"SELECT s.id, s.event_id, s.title AS "title!: Value"
             FROM programme.sessions s
            WHERE s.status = 'live' AND ($1::uuid IS NULL OR s.id <> $1)
            ORDER BY s.live_started_at DESC NULLS LAST
            LIMIT 1"#,
        sauf
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne.map(|l| DirectEnCours {
        session_id: l.id,
        event_id: l.event_id,
        title: l.title,
    }))
}

pub struct RaisonDAnnulation {
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
}

pub async fn raisons_dannulation<'e>(
    executor: impl PgExecutor<'e>,
) -> Result<Vec<RaisonDAnnulation>> {
    let lignes = sqlx::query!(
        r#"SELECT code, label AS "label!: Value", description AS "description?: Value"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'session_cancellation_reason' AND is_active
            ORDER BY sort_order, code"#
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| RaisonDAnnulation {
            code: l.code,
            label: l.label,
            description: l.description,
        })
        .collect())
}
