//! Les intervenants en ligne (08/11) : ce que la régie et le lien d'inscription
//! en lisent hors schéma. `event.events` (titre, ville), `identity.people`
//! (nom, adresse), `live.meetings` et `live.meeting_participants` (le lien
//! Zoom demandé), `platform.jobs` (le lien d'inscription parti au porteur).
//! Lecture seule.

use contracts::engagement::SEND_SPEAKER_SIGNUP_LINK;
use kernel::error::Result;
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SeanceDuLien {
    pub id: Uuid,
    pub event_id: Uuid,
    pub title: Value,
    pub event_title: Value,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub timezone: String,
    pub city: Option<String>,
    pub status: String,
    pub is_published: bool,
    /// La séance, et sa journée si elle en a une, se suivent en ligne.
    pub is_streamed: bool,
    pub token: String,
}

impl SeanceDuLien {
    /// Le lien sert jusqu'à la fin de l'activité, si elle se suit en ligne.
    pub fn ouverte(&self, maintenant: OffsetDateTime) -> bool {
        self.is_published
            && self.is_streamed
            && self.status != "cancelled"
            && self.ends_at > maintenant
    }
}

pub async fn seance_par_jeton<'e>(
    executor: impl PgExecutor<'e>,
    jeton: &str,
) -> Result<Option<SeanceDuLien>> {
    let ligne = sqlx::query_as!(
        SeanceDuLien,
        r#"SELECT s.id, s.event_id, s.title AS "title!: Value", e.title AS "event_title!: Value",
                  s.starts_at, s.ends_at, s.timezone::text AS "timezone!", e.city,
                  s.status::text AS "status!", (s.published_at IS NOT NULL) AS "is_published!",
                  (s.is_streamed AND COALESCE((SELECT t.is_streamed FROM event.programme_tracks t
                                                WHERE t.id = programme.day_track_of(s.id)), true))
                      AS "is_streamed!",
                  s.speaker_link_token AS token
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.speaker_link_token = $1"#,
        jeton
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

pub async fn seance_par_id<'e>(
    executor: impl PgExecutor<'e>,
    session_id: Uuid,
) -> Result<Option<SeanceDuLien>> {
    let ligne = sqlx::query_as!(
        SeanceDuLien,
        r#"SELECT s.id, s.event_id, s.title AS "title!: Value", e.title AS "event_title!: Value",
                  s.starts_at, s.ends_at, s.timezone::text AS "timezone!", e.city,
                  s.status::text AS "status!", (s.published_at IS NOT NULL) AS "is_published!",
                  (s.is_streamed AND COALESCE((SELECT t.is_streamed FROM event.programme_tracks t
                                                WHERE t.id = programme.day_track_of(s.id)), true))
                      AS "is_streamed!",
                  s.speaker_link_token AS token
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.id = $1"#,
        session_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

#[derive(Debug, Clone)]
pub struct IntervenantDeRegie {
    pub id: Uuid,
    pub display_name: String,
    pub job_title: Option<String>,
    pub organization: Option<String>,
    pub email: Option<String>,
    pub source: String,
    pub is_listed: bool,
    pub created_at: OffsetDateTime,
    pub link_sent_at: Option<OffsetDateTime>,
}

/// Les intervenants d'une séance, et pour chacun l'instant où son lien Zoom a
/// été demandé dans la réunion qui la diffuse (la sienne ou celle de sa journée).
pub async fn intervenants_de<'e>(
    executor: impl PgExecutor<'e>,
    session_id: Uuid,
) -> Result<Vec<IntervenantDeRegie>> {
    let lignes = sqlx::query_as!(
        IntervenantDeRegie,
        r#"SELECT d.id,
                  COALESCE(p.display_name, btrim(d.first_name || ' ' || d.last_name)) AS "display_name!",
                  d.job_title_snapshot AS job_title, d.organization_snapshot AS organization,
                  p.primary_email::text AS "email?", d.source, d.is_listed, d.created_at,
                  (SELECT max(mp.link_requested_at)
                     FROM live.meeting_participants mp
                     JOIN live.meetings m ON m.id = mp.meeting_id
                    WHERE (m.session_id = d.session_id OR m.track_id = programme.day_track_of(d.session_id))
                      AND m.status NOT IN ('cancelled', 'failed')
                      AND mp.email = p.primary_email) AS "link_sent_at?"
             FROM programme.session_speakers d
             LEFT JOIN identity.people p ON p.id = d.person_id AND p.status <> 'anonymized'
            WHERE d.session_id = $1
            ORDER BY d.sort_order, d.created_at"#,
        session_id
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes)
}

/// Les adresses du porteur : déposant et personne à contacter de chaque dossier
/// de la séance (une séance commune en a plusieurs).
pub async fn porteurs<'e>(executor: impl PgExecutor<'e>, session_id: Uuid) -> Result<Vec<String>> {
    let adresses = sqlx::query_scalar!(
        r#"SELECT DISTINCT lower(p.primary_email::text) AS "email!"
             FROM programme.proposals pr
             JOIN identity.people p ON p.id IN (pr.submitted_by, pr.contact_person_id)
            WHERE p.status <> 'anonymized'
              AND (pr.id = (SELECT s.proposal_id FROM programme.sessions s WHERE s.id = $1)
                   OR pr.id IN (SELECT sp.proposal_id FROM programme.session_proposals sp
                                 WHERE sp.session_id = $1))
            ORDER BY 1"#,
        session_id
    )
    .fetch_all(executor)
    .await?;
    Ok(adresses)
}

#[derive(Debug, Clone)]
pub struct ResumeDesIntervenants {
    pub session_id: Uuid,
    pub online: i64,
    pub reachable: i64,
    pub to_list: i64,
    pub links_sent_at: Option<OffsetDateTime>,
    pub owner_link_sent_at: Option<OffsetDateTime>,
    pub has_owner: bool,
}

pub async fn resumes<'e>(
    executor: impl PgExecutor<'e>,
    seances: &[Uuid],
) -> Result<Vec<ResumeDesIntervenants>> {
    let lignes = sqlx::query_as!(
        ResumeDesIntervenants,
        r#"WITH liens AS (
               SELECT d.session_id, d.source, d.is_listed, p.primary_email AS email,
                      (SELECT max(mp.link_requested_at)
                         FROM live.meeting_participants mp
                         JOIN live.meetings m ON m.id = mp.meeting_id
                        WHERE (m.session_id = d.session_id OR m.track_id = programme.day_track_of(d.session_id))
                          AND m.status NOT IN ('cancelled', 'failed')
                          AND mp.email = p.primary_email) AS demande
                 FROM programme.session_speakers d
                 LEFT JOIN identity.people p ON p.id = d.person_id AND p.status <> 'anonymized'
                WHERE d.session_id = ANY($1)
           )
           SELECT s.id AS "session_id!",
                  (SELECT count(*) FROM liens l WHERE l.session_id = s.id AND l.demande IS NOT NULL) AS "online!",
                  (SELECT count(*) FROM liens l WHERE l.session_id = s.id AND l.email IS NOT NULL) AS "reachable!",
                  (SELECT count(*) FROM liens l WHERE l.session_id = s.id AND l.source = 'lien' AND NOT l.is_listed) AS "to_list!",
                  (SELECT max(l.demande) FROM liens l WHERE l.session_id = s.id) AS "links_sent_at?",
                  (SELECT max(j.completed_at) FROM platform.jobs j
                    WHERE j.task = $2 AND j.status = 'succeeded'
                      AND j.payload ->> 'session_id' = s.id::text
                      AND j.payload ->> 'preview_to' IS NULL) AS "owner_link_sent_at?",
                  EXISTS (SELECT 1 FROM programme.proposals pr
                           WHERE pr.id = s.proposal_id
                              OR pr.id IN (SELECT sp.proposal_id FROM programme.session_proposals sp
                                            WHERE sp.session_id = s.id)) AS "has_owner!"
             FROM programme.sessions s
            WHERE s.id = ANY($1)"#,
        seances,
        SEND_SPEAKER_SIGNUP_LINK
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes)
}
