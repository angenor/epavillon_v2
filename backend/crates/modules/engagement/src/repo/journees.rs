//! Lecture de `programme.track_registrations` et de sa journée (02/11), en
//! lecture seule : de quoi composer la confirmation et les rappels de la
//! journée. L'heure de début se lit sur le déroulé (activités non annulées et
//! moments), comme sur la page de la journée, et se formate dans le fuseau de
//! l'édition par la base, pour la même raison que `seance_pour_rappel`.

use kernel::error::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JourneePourCourriel {
    pub track_registration_id: Uuid,
    pub status: String,
    pub person_id: Uuid,
    pub first_name: String,
    pub email: String,
    /// Langue choisie à l'inscription.
    pub locale: String,
    pub event_id: Uuid,
    pub titre: String,
    pub event_slug: String,
    pub track_slug: String,
    pub timezone: String,
    pub debut: Option<OffsetDateTime>,
    pub debut_local: Option<String>,
    pub gardees: i64,
    pub activites: i64,
}

pub async fn journee_pour_courriel(
    pool: &PgPool,
    track_registration_id: Uuid,
) -> Result<Option<JourneePourCourriel>> {
    let ligne = sqlx::query!(
        r#"WITH tr AS (
               SELECT * FROM programme.track_registrations WHERE id = $1
           ), debut AS (
               SELECT min(x.d) AS d FROM (
                   SELECT s.starts_at AS d
                     FROM programme.session_tracks st
                     JOIN programme.sessions s ON s.id = st.session_id
                    WHERE st.track_id = (SELECT track_id FROM tr) AND s.status <> 'cancelled'
                   UNION ALL
                   SELECT m.starts_at FROM event.programme_track_moments m
                    WHERE m.track_id = (SELECT track_id FROM tr)) x
           )
           SELECT tr.id, tr.status::text AS "status!", tr.person_id, p.first_name,
                  p.primary_email::text AS "email!", tr.locale, t.event_id,
                  platform.t(t.title, tr.locale) AS "titre!",
                  e.slug::text AS "event_slug!", t.slug::text AS "track_slug!",
                  e.timezone::text AS "timezone!",
                  debut.d AS debut,
                  to_char(debut.d AT TIME ZONE e.timezone,
                          CASE WHEN tr.locale LIKE 'en%' THEN 'YYYY-MM-DD "at" HH24:MI'
                               ELSE 'DD/MM/YYYY "à" HH24"h"MI' END) AS debut_local,
                  (SELECT count(*) FROM programme.registrations r
                    WHERE r.track_registration_id = tr.id AND r.status <> 'cancelled')
                      AS "gardees!",
                  (SELECT count(*) FROM programme.session_tracks st
                     JOIN programme.sessions s ON s.id = st.session_id
                    WHERE st.track_id = tr.track_id AND s.published_at IS NOT NULL
                      AND s.status <> 'cancelled') AS "activites!"
             FROM tr
             JOIN identity.people p ON p.id = tr.person_id
             JOIN event.programme_tracks t ON t.id = tr.track_id
             JOIN event.events e ON e.id = t.event_id
             CROSS JOIN debut"#,
        track_registration_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| JourneePourCourriel {
        track_registration_id: l.id,
        status: l.status,
        person_id: l.person_id,
        first_name: l.first_name,
        email: l.email,
        locale: l.locale,
        event_id: l.event_id,
        titre: l.titre,
        event_slug: l.event_slug,
        track_slug: l.track_slug,
        timezone: l.timezone,
        debut: l.debut,
        debut_local: l.debut_local,
        gardees: l.gardees,
        activites: l.activites,
    }))
}

/// Les décalages d'un jour ou plus de la règle de l'édition, en minutes, si
/// elle est active et envoie des courriels. Ceux de moins d'un jour partent
/// par activité gardée (`engagement.schedule_session_reminders`).
pub async fn decalages_de_journee(pool: &PgPool, event_id: Uuid) -> Result<Vec<i64>> {
    let minutes = sqlx::query_scalar!(
        r#"SELECT (extract(epoch FROM o.decalage) / 60)::bigint AS "minutes!"
             FROM engagement.reminder_rules r
            CROSS JOIN unnest(r.offsets) AS o(decalage)
            WHERE r.event_id = $1 AND r.is_active AND 'email' = ANY (r.channels)
              AND o.decalage >= interval '1 day'
            ORDER BY 1 DESC"#,
        event_id
    )
    .fetch_all(pool)
    .await?;
    Ok(minutes)
}

/// Les inscriptions actives à la journée dont l'activité suit le direct.
pub async fn inscriptions_de_la_journee_de(pool: &PgPool, session_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT tr.id FROM programme.track_registrations tr
          WHERE tr.track_id = programme.day_track_of($1)
            AND tr.status IN ('registered', 'attended')",
        session_id
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}
