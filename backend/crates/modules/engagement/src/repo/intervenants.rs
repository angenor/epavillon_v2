//! Les courriels des intervenants en ligne (08/11) : ce qu'ils lisent hors
//! schéma. `programme.sessions` et `event.events` (titre, heures, ville, jeton
//! du lien), `programme.proposals` et `identity.people` (le porteur),
//! `live.meeting_participants` (le lien Zoom), `platform.settings` (la copie).
//! Lecture seule.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub struct ActiviteDuLien {
    pub event_id: Uuid,
    pub acronym: Option<String>,
    pub title: Value,
    /// Heures dans le fuseau de la séance.
    pub debut: PrimitiveDateTime,
    pub fin: PrimitiveDateTime,
    pub timezone: String,
    pub city: Option<String>,
    pub token: String,
}

pub async fn activite(pool: &PgPool, session_id: Uuid) -> Result<Option<ActiviteDuLien>> {
    let ligne = sqlx::query_as!(
        ActiviteDuLien,
        r#"SELECT s.event_id, e.acronym, s.title AS "title!: Value",
                  (s.starts_at AT TIME ZONE s.timezone) AS "debut!",
                  (s.ends_at AT TIME ZONE s.timezone) AS "fin!",
                  s.timezone::text AS "timezone!", e.city, s.speaker_link_token AS token
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.id = $1 AND s.status <> 'cancelled'"#,
        session_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne)
}

pub struct Destinataire {
    pub email: String,
    pub civility: Option<String>,
    pub last_name: String,
    pub locale: String,
}

/// Le déposant et la personne à contacter de chaque dossier de la séance.
pub async fn porteurs(pool: &PgPool, session_id: Uuid) -> Result<Vec<Destinataire>> {
    let lignes = sqlx::query_as!(
        Destinataire,
        r#"SELECT DISTINCT ON (lower(p.primary_email::text))
                  lower(p.primary_email::text) AS "email!", p.civility, p.last_name,
                  COALESCE(p.preferred_locale::text, 'fr') AS "locale!"
             FROM programme.proposals pr
             JOIN identity.people p ON p.id IN (pr.submitted_by, pr.contact_person_id)
            WHERE p.status <> 'anonymized'
              AND (pr.id = (SELECT s.proposal_id FROM programme.sessions s WHERE s.id = $1)
                   OR pr.id IN (SELECT sp.proposal_id FROM programme.session_proposals sp
                                 WHERE sp.session_id = $1))
            ORDER BY lower(p.primary_email::text), (p.id = pr.submitted_by) DESC"#,
        session_id
    )
    .fetch_all(pool)
    .await?;
    Ok(lignes)
}

pub struct PlaceAvecLien {
    pub email: String,
    pub civility: Option<String>,
    pub last_name: String,
    pub locale: String,
    pub person_id: Option<Uuid>,
    pub join_url: String,
}

/// La place d'un intervenant, si son lien Zoom existe et qu'elle vit encore.
pub async fn place(pool: &PgPool, participant_id: Uuid) -> Result<Option<PlaceAvecLien>> {
    let ligne = sqlx::query_as!(
        PlaceAvecLien,
        r#"SELECT mp.email::text AS "email!", p.civility,
                  COALESCE(p.last_name, mp.display_name) AS "last_name!",
                  COALESCE(p.preferred_locale::text, 'fr') AS "locale!",
                  mp.person_id, mp.personal_join_url::text AS "join_url!"
             FROM live.meeting_participants mp
             LEFT JOIN identity.people p ON p.id = mp.person_id
            WHERE mp.id = $1 AND mp.cancelled_at IS NULL
              AND mp.sync_status = 'synced' AND mp.personal_join_url IS NOT NULL"#,
        participant_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne)
}

/// La copie de chaque courriel d'intervenant ; un réglage absent n'en met aucune.
pub async fn copie(pool: &PgPool) -> Result<Vec<String>> {
    let adresses = sqlx::query_scalar!(
        r#"SELECT ARRAY(SELECT jsonb_array_elements_text(value)) AS adresses
             FROM platform.settings
            WHERE key = 'engagement.speaker_links.cc' AND jsonb_typeof(value) = 'array'"#
    )
    .fetch_optional(pool)
    .await?;
    Ok(adresses.flatten().unwrap_or_default())
}
