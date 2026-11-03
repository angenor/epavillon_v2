//! Les courriels programmés (04/11) : les rappels en file, regroupés par envoi,
//! et leurs textes (`engagement.scheduled_email_texts`).
//!
//! Lit `platform.jobs`, les inscriptions (`programme`) et leurs cibles
//! (`programme.sessions`, `event.programme_tracks`) sans y écrire. Un rappel de
//! journée dont la journée a bougé ne part pas (son travail vise l'ancienne
//! heure) : la liste l'écarte, comme le travail.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::{OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::domain::programmes::{
    CleEnvoi, CleType, Nature, ScheduledMailBatchRow, ScheduledMailText,
};

/// Les envois à venir d'une édition, et pour chacun si son type a un texte.
pub async fn envois(
    pool: &PgPool,
    event_id: Uuid,
    locale: &str,
) -> Result<Vec<(ScheduledMailBatchRow, bool)>> {
    let lignes = sqlx::query!(
        r#"WITH activites AS (
               SELECT sr.session_id AS cible,
                      round(extract(epoch FROM sr.offset_before) / 60)::int AS decalage,
                      NULL::timestamptz AS debut,
                      min(j.run_at) AS depart,
                      count(*) AS destinataires,
                      platform.t(s.title, $2::text) AS titre
                 FROM platform.jobs j
                 JOIN engagement.scheduled_reminders sr ON sr.job_id = j.id
                 JOIN programme.sessions s ON s.id = sr.session_id
                WHERE j.task = 'engagement.send_reminder' AND j.status = 'queued'
                  AND j.run_at > now()
                  AND sr.status IN ('pending', 'queued') AND sr.channel = 'email'
                  AND s.event_id = $1::uuid
                GROUP BY s.id, sr.session_id, 2
           ), journees AS (
               SELECT tr.track_id AS cible,
                      (j.payload ->> 'offset_minutes')::int AS decalage,
                      to_timestamp((j.payload ->> 'starts_at')::bigint) AS debut,
                      min(j.run_at) AS depart,
                      count(*) AS destinataires,
                      platform.t(t.title, $2::text) AS titre
                 FROM platform.jobs j
                 JOIN programme.track_registrations tr
                   ON tr.id = (j.payload ->> 'track_registration_id')::uuid
                 JOIN event.programme_tracks t ON t.id = tr.track_id
                 JOIN LATERAL (
                     SELECT min(x.d) AS d FROM (
                         SELECT s.starts_at AS d
                           FROM programme.session_tracks st
                           JOIN programme.sessions s ON s.id = st.session_id
                          WHERE st.track_id = t.id AND s.status <> 'cancelled'
                         UNION ALL
                         SELECT m.starts_at FROM event.programme_track_moments m
                          WHERE m.track_id = t.id) x
                 ) actuel ON extract(epoch FROM actuel.d)::bigint = (j.payload ->> 'starts_at')::bigint
                WHERE j.task = 'engagement.send_day_reminder' AND j.status = 'queued'
                  AND j.run_at > now()
                  AND tr.status IN ('registered', 'attended')
                  AND t.event_id = $1::uuid
                GROUP BY t.id, tr.track_id, 2, 3
           ), envois AS (
               SELECT 'activity'::text AS nature, a.* FROM activites a
               UNION ALL
               SELECT 'day'::text, g.* FROM journees g
           )
           SELECT e.nature AS "nature!", e.cible AS "cible!", e.decalage AS "decalage!",
                  e.debut AS "debut?", e.depart AS "depart!",
                  e.destinataires AS "destinataires!", e.titre AS "titre!",
                  EXISTS (SELECT 1 FROM engagement.scheduled_email_texts x
                           WHERE x.event_id = $1::uuid AND x.kind = e.nature
                             AND x.offset_minutes = e.decalage
                             AND coalesce(x.session_id, x.track_id) = e.cible
                             AND x.target_starts_at IS NOT DISTINCT FROM e.debut) AS "propre!",
                  EXISTS (SELECT 1 FROM engagement.scheduled_email_texts x
                           WHERE x.event_id = $1::uuid AND x.kind = e.nature
                             AND x.session_id IS NULL AND x.track_id IS NULL
                             AND x.offset_minutes IS NOT DISTINCT FROM
                                 (CASE WHEN e.nature = 'activity' THEN e.decalage END)) AS "type_ecrit!"
             FROM envois e
            ORDER BY e.depart, e.titre"#,
        event_id,
        locale
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            let nature = Nature::depuis(&l.nature)?;
            let cle = CleEnvoi {
                nature,
                cible: l.cible,
                offset_minutes: l.decalage,
                debut: l.debut.map(OffsetDateTime::unix_timestamp),
            };
            Some((
                ScheduledMailBatchRow {
                    key: cle.texte(),
                    type_key: cle.type_().texte(),
                    kind: l.nature,
                    offset_minutes: l.decalage,
                    target_id: l.cible,
                    target_starts_at: l.debut,
                    title: l.titre,
                    run_at: l.depart,
                    recipients: l.destinataires,
                    has_own_text: l.propre,
                },
                l.type_ecrit,
            ))
        })
        .collect())
}

/// Le premier destinataire d'un envoi, celui de l'aperçu.
#[derive(Debug, Clone)]
pub struct PremierDestinataire {
    pub first_name: String,
    pub email: String,
    pub locale: String,
}

pub async fn premier_destinataire(
    pool: &PgPool,
    cle: &CleEnvoi,
) -> Result<Option<PremierDestinataire>> {
    match cle.nature {
        Nature::Activite => {
            let ligne = sqlx::query!(
                r#"SELECT p.first_name AS "first_name!", p.primary_email::text AS "email!",
                          p.preferred_locale::text AS "locale!"
                     FROM platform.jobs j
                     JOIN engagement.scheduled_reminders sr ON sr.job_id = j.id
                     JOIN identity.people p ON p.id = sr.person_id
                    WHERE j.task = 'engagement.send_reminder' AND j.status = 'queued'
                      AND j.run_at > now()
                      AND sr.status IN ('pending', 'queued') AND sr.channel = 'email'
                      AND sr.session_id = $1::uuid
                      AND round(extract(epoch FROM sr.offset_before) / 60)::int = $2::int
                    ORDER BY j.run_at, p.last_name, p.first_name, sr.id
                    LIMIT 1"#,
                cle.cible,
                cle.offset_minutes
            )
            .fetch_optional(pool)
            .await?;
            Ok(ligne.map(|l| PremierDestinataire {
                first_name: l.first_name,
                email: l.email,
                locale: l.locale,
            }))
        }
        Nature::Journee => {
            let ligne = sqlx::query!(
                r#"SELECT p.first_name AS "first_name!", p.primary_email::text AS "email!",
                          tr.locale AS "locale!"
                     FROM platform.jobs j
                     JOIN programme.track_registrations tr
                       ON tr.id = (j.payload ->> 'track_registration_id')::uuid
                     JOIN identity.people p ON p.id = tr.person_id
                    WHERE j.task = 'engagement.send_day_reminder' AND j.status = 'queued'
                      AND j.run_at > now()
                      AND tr.status IN ('registered', 'attended')
                      AND tr.track_id = $1::uuid
                      AND (j.payload ->> 'offset_minutes')::int = $2::int
                      AND (j.payload ->> 'starts_at')::bigint = $3::bigint
                    ORDER BY j.run_at, p.last_name, p.first_name, tr.id
                    LIMIT 1"#,
                cle.cible,
                cle.offset_minutes,
                cle.debut.unwrap_or_default()
            )
            .fetch_optional(pool)
            .await?;
            Ok(ligne.map(|l| PremierDestinataire {
                first_name: l.first_name,
                email: l.email,
                locale: l.locale,
            }))
        }
    }
}

/// L'activité ou la journée d'un rappel, de quoi remplir ses variables.
#[derive(Debug, Clone)]
pub struct CibleRappel {
    pub event_id: Uuid,
    pub cible: Uuid,
    pub titre: String,
    pub slug: String,
    pub event_slug: String,
    pub edition: String,
    pub timezone: String,
    pub ville: Option<String>,
    /// Le début, dans le fuseau de la cible.
    pub debut_local: PrimitiveDateTime,
}

pub async fn cible_activite(
    pool: &PgPool,
    session_id: Uuid,
    locale: &str,
) -> Result<Option<CibleRappel>> {
    let ligne = sqlx::query!(
        r#"SELECT s.event_id AS "event_id!", s.id AS "cible!",
                  platform.t(s.title, $2::text) AS "titre!",
                  s.slug::text AS "slug!", e.slug::text AS "event_slug!",
                  platform.t(e.title, $2::text) AS "edition!",
                  s.timezone::text AS "timezone!", e.city AS "ville?",
                  (s.starts_at AT TIME ZONE s.timezone) AS "debut_local!"
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.id = $1::uuid"#,
        session_id,
        locale
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne.map(|l| CibleRappel {
        event_id: l.event_id,
        cible: l.cible,
        titre: l.titre,
        slug: l.slug,
        event_slug: l.event_slug,
        edition: l.edition,
        timezone: l.timezone,
        ville: l.ville,
        debut_local: l.debut_local,
    }))
}

/// La journée, par son identifiant ou par une inscription, au début visé.
pub async fn cible_journee(
    pool: &PgPool,
    track_id: Option<Uuid>,
    track_registration_id: Option<Uuid>,
    debut: OffsetDateTime,
    locale: &str,
) -> Result<Option<CibleRappel>> {
    let ligne = sqlx::query!(
        r#"SELECT t.event_id AS "event_id!", t.id AS "cible!",
                  platform.t(t.title, $4::text) AS "titre!",
                  t.slug::text AS "slug!", e.slug::text AS "event_slug!",
                  platform.t(e.title, $4::text) AS "edition!",
                  e.timezone::text AS "timezone!", e.city AS "ville?",
                  ($3::timestamptz AT TIME ZONE e.timezone) AS "debut_local!"
             FROM event.programme_tracks t
             JOIN event.events e ON e.id = t.event_id
            WHERE t.id = coalesce($1::uuid,
                                  (SELECT tr.track_id FROM programme.track_registrations tr
                                    WHERE tr.id = $2::uuid))"#,
        track_id,
        track_registration_id,
        debut,
        locale
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne.map(|l| CibleRappel {
        event_id: l.event_id,
        cible: l.cible,
        titre: l.titre,
        slug: l.slug,
        event_slug: l.event_slug,
        edition: l.edition,
        timezone: l.timezone,
        ville: l.ville,
        debut_local: l.debut_local,
    }))
}

/// Un texte enregistré : sa langue, et s'il est propre à l'envoi.
#[derive(Debug, Clone)]
pub struct LigneTexte {
    pub locale: String,
    pub texte: ScheduledMailText,
    pub propre: bool,
}

/// Les textes du type et de l'envoi, dans une langue ou les deux.
pub async fn textes(
    pool: &PgPool,
    event_id: Uuid,
    cle: &CleEnvoi,
    langue: Option<&str>,
) -> Result<Vec<LigneTexte>> {
    let lignes = sqlx::query!(
        r#"SELECT t.locale AS "locale!", t.subject AS "subject!", t.body AS "body!",
                  (t.session_id IS NOT NULL OR t.track_id IS NOT NULL) AS "propre!"
             FROM engagement.scheduled_email_texts t
            WHERE t.event_id = $1::uuid AND t.kind = $2::text
              AND ($3::text IS NULL OR t.locale = $3::text)
              AND ((t.session_id IS NULL AND t.track_id IS NULL
                    AND t.offset_minutes IS NOT DISTINCT FROM $4::int)
                   OR (t.offset_minutes = $5::int
                       AND coalesce(t.session_id, t.track_id) = $6::uuid
                       AND t.target_starts_at IS NOT DISTINCT FROM $7::timestamptz))"#,
        event_id,
        cle.nature.code(),
        langue,
        cle.type_().offset_minutes,
        cle.offset_minutes,
        cle.cible,
        cle.debut_vise()
    )
    .fetch_all(pool)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| LigneTexte {
            locale: l.locale,
            texte: ScheduledMailText {
                subject: l.subject,
                body: l.body,
            },
            propre: l.propre,
        })
        .collect())
}

/// Où un texte s'écrit : le type de l'édition, ou un envoi.
pub enum Portee<'a> {
    Type(&'a CleType),
    Envoi(&'a CleEnvoi),
}

pub async fn ecrire(
    conn: &mut PgConnection,
    event_id: Uuid,
    portee: Portee<'_>,
    langue: &str,
    texte: &ScheduledMailText,
    acteur: Uuid,
) -> Result<()> {
    let (nature, decalage, seance, journee, debut) = match portee {
        Portee::Type(t) => (t.nature, t.offset_minutes, None, None, None),
        Portee::Envoi(e) => {
            let (seance, journee) = match e.nature {
                Nature::Activite => (Some(e.cible), None),
                Nature::Journee => (None, Some(e.cible)),
            };
            (
                e.nature,
                Some(e.offset_minutes),
                seance,
                journee,
                e.debut_vise(),
            )
        }
    };
    sqlx::query!(
        r#"INSERT INTO engagement.scheduled_email_texts
               (event_id, kind, offset_minutes, session_id, track_id, target_starts_at,
                locale, subject, body, updated_by)
           VALUES ($1::uuid, $2::text, $3::int, $4::uuid, $5::uuid, $6::timestamptz,
                   $7::text, $8::text, $9::text, $10::uuid)
           ON CONFLICT ON CONSTRAINT ux_scheduled_email_texts
           DO UPDATE SET subject = EXCLUDED.subject, body = EXCLUDED.body,
                         updated_by = EXCLUDED.updated_by"#,
        event_id,
        nature.code(),
        decalage,
        seance,
        journee,
        debut,
        langue,
        texte.subject,
        texte.body,
        acteur
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// « Revenir au texte du type » : efface le texte propre à l'envoi.
pub async fn effacer_envoi(conn: &mut PgConnection, event_id: Uuid, cle: &CleEnvoi) -> Result<u64> {
    let fait = sqlx::query!(
        r#"DELETE FROM engagement.scheduled_email_texts t
            WHERE t.event_id = $1::uuid AND t.kind = $2::text AND t.offset_minutes = $3::int
              AND coalesce(t.session_id, t.track_id) = $4::uuid
              AND t.target_starts_at IS NOT DISTINCT FROM $5::timestamptz"#,
        event_id,
        cle.nature.code(),
        cle.offset_minutes,
        cle.cible,
        cle.debut_vise()
    )
    .execute(&mut *conn)
    .await?;
    Ok(fait.rows_affected())
}
