//! Ce que le courriel de changement relit au moment de partir (research R9) :
//! la tranche de la base, la personne, son accord, et l'état final de la cible.
//! Les heures sortent déjà dans le fuseau de la COP.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

pub const TRANCHE_SECONDES: i64 = 600;

/// La tranche de dix minutes en cours, à l'horloge de la base.
pub async fn tranche(conn: &mut PgConnection) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT (floor(extract(epoch FROM now()) / $1::float8))::bigint AS "n!""#,
        TRANCHE_SECONDES as f64
    )
    .fetch_one(conn)
    .await?)
}

pub struct Destinataire {
    pub email: String,
    pub locale: String,
}

pub async fn destinataire(conn: &mut PgConnection, id: Uuid) -> Result<Option<Destinataire>> {
    let ligne = sqlx::query!(
        r#"SELECT primary_email::text AS "email!", preferred_locale
             FROM identity.people WHERE id = $1 AND status = 'active'"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Destinataire {
        email: l.email,
        locale: l.preferred_locale,
    }))
}

pub async fn suit_la_session(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    person: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT $2 IN (SELECT negotiation.change_recipients($1)) AS "suit!""#,
        meeting_id,
        person
    )
    .fetch_one(conn)
    .await?)
}

pub async fn suit_la_reunion(conn: &mut PgConnection, id: Uuid, person: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT $2 IN (SELECT negotiation.network_recipients($1)) AS "suit!""#,
        id,
        person
    )
    .fetch_one(conn)
    .await?)
}

pub struct EtatSession {
    pub title_en: String,
    pub title_fr: Option<String>,
    pub annulee: bool,
    pub jour: Date,
    pub debut: String,
    pub fin: Option<String>,
    pub salle: Option<String>,
    pub ville: Option<String>,
    pub fuseau: String,
    /// Champs changés par l'import dans la tranche : `start`, `venue`, `status`.
    pub changes: Vec<String>,
    /// Signalements **publiés** dans la tranche, non retirés.
    pub signalements: Vec<SignalementPublie>,
}

pub struct SignalementPublie {
    pub reason: String,
    pub heure: Option<String>,
    pub salle: Option<String>,
}

pub async fn etat_session(
    conn: &mut PgConnection,
    id: Uuid,
    debut: OffsetDateTime,
    fin: OffsetDateTime,
) -> Result<Option<EtatSession>> {
    let Some(m) = sqlx::query!(
        r#"SELECT COALESCE(m.title_original, m.title->>'fr') AS "title_en!", tr.text_fr AS "title_fr?",
                  m.status::text = 'cancelled' AS "annulee!",
                  (m.start_at AT TIME ZONE e.timezone::text)::date AS "jour!",
                  to_char(m.start_at AT TIME ZONE e.timezone::text, 'HH24:MI') AS "debut!",
                  to_char(m.end_at AT TIME ZONE e.timezone::text, 'HH24:MI') AS fin,
                  m.venue_label, e.city, e.timezone::text AS "fuseau!"
             FROM negotiation.meetings m
             JOIN event.events e ON e.id = m.event_id
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
            WHERE m.id = $1"#,
        id
    )
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(None);
    };

    let changes = sqlx::query_scalar!(
        r#"SELECT DISTINCT field AS "field!" FROM negotiation.meeting_changes
            WHERE meeting_id = $1 AND field IN ('start', 'venue', 'status')
              AND detected_at >= $2 AND detected_at < $3"#,
        id,
        debut,
        fin
    )
    .fetch_all(&mut *conn)
    .await?;

    let signalements = sqlx::query!(
        r#"SELECT r.reason,
                  to_char(r.proposed_start AT TIME ZONE e.timezone::text, 'HH24:MI') AS heure,
                  r.proposed_venue
             FROM negotiation.session_reports r
             JOIN event.events e ON e.id = r.event_id
            WHERE r.meeting_id = $1 AND r.published_at >= $2 AND r.published_at < $3
              AND r.withdrawn_at IS NULL
            ORDER BY r.published_at"#,
        id,
        debut,
        fin
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|l| SignalementPublie {
        reason: l.reason,
        heure: l.heure,
        salle: l.proposed_venue,
    })
    .collect();

    Ok(Some(EtatSession {
        title_en: m.title_en,
        title_fr: m.title_fr,
        annulee: m.annulee,
        jour: m.jour,
        debut: m.debut,
        fin: m.fin,
        salle: m.venue_label,
        ville: m.city,
        fuseau: m.fuseau,
        changes,
        signalements,
    }))
}

pub struct EtatReunion {
    pub titre: String,
    pub jour: Date,
    pub heure: Option<String>,
    pub lieu: Option<String>,
    pub ville: Option<String>,
    pub fuseau: String,
}

/// Retirée, ou jamais publiée : rien.
pub async fn etat_reunion(conn: &mut PgConnection, id: Uuid) -> Result<Option<EtatReunion>> {
    let ligne = sqlx::query!(
        r#"SELECT nm.title, nm.day,
                  to_char(nm.start_at AT TIME ZONE e.timezone::text, 'HH24:MI') AS heure,
                  nm.venue, e.city, e.timezone::text AS "fuseau!"
             FROM negotiation.network_meetings nm
             JOIN event.events e ON e.id = nm.event_id
             JOIN negotiation.session_reports r ON r.id = nm.report_id
            WHERE nm.id = $1 AND nm.withdrawn_at IS NULL AND r.published_at IS NOT NULL"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| EtatReunion {
        titre: l.title,
        jour: l.day,
        heure: l.heure,
        lieu: l.venue,
        ville: l.city,
        fuseau: l.fuseau,
    }))
}

/// Ce que l'avis d'un changement importé doit dire de la session.
pub struct SessionPourAvis {
    pub title_en: String,
    pub title_fr: Option<String>,
    pub heure: String,
    pub salle: Option<String>,
    pub ville: Option<String>,
    pub jour: Date,
}

pub async fn session_pour_avis(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<SessionPourAvis>> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(m.title_original, m.title->>'fr') AS "title_en!", tr.text_fr AS "title_fr?",
                  to_char(m.start_at AT TIME ZONE e.timezone::text, 'HH24:MI') AS "heure!",
                  m.venue_label, e.city, (now() AT TIME ZONE e.timezone::text)::date AS "jour!"
             FROM negotiation.meetings m
             JOIN event.events e ON e.id = m.event_id
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
            WHERE m.id = $1"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| SessionPourAvis {
        title_en: l.title_en,
        title_fr: l.title_fr,
        heure: l.heure,
        salle: l.venue_label,
        ville: l.city,
        jour: l.jour,
    }))
}

// ---------------------------------------------------------------------------
// Réunions de la Francophonie (étape 4) — relues après écriture, par l'avis
// comme par le courriel.
// ---------------------------------------------------------------------------

pub struct ReunionFrancophone {
    pub title_fr: String,
    pub title_en: String,
    pub status: String,
    pub jour: Date,
    pub debut: String,
    pub fin: Option<String>,
    pub format: String,
    pub lieu: Option<String>,
    pub motif: Option<String>,
    pub ville: Option<String>,
    pub fuseau: String,
    /// Aujourd'hui dans le fuseau de la COP : le jour de la clé de regroupement.
    pub aujourdhui: Date,
}

pub async fn reunion_francophone(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<ReunionFrancophone>> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(m.title::jsonb->>'fr', m.title::jsonb->>'en') AS "title_fr!",
                  COALESCE(m.title::jsonb->>'en', m.title::jsonb->>'fr') AS "title_en!",
                  m.status::text AS "status!",
                  (m.start_at AT TIME ZONE m.timezone::text)::date AS "jour!",
                  to_char(m.start_at AT TIME ZONE m.timezone::text, 'HH24:MI') AS "debut!",
                  to_char(m.end_at AT TIME ZONE m.timezone::text, 'HH24:MI') AS fin,
                  m.format::text AS "format!", m.venue_label, m.cancellation_reason, e.city,
                  m.timezone::text AS "fuseau!",
                  (now() AT TIME ZONE m.timezone::text)::date AS "aujourdhui!"
             FROM negotiation.meetings m
             JOIN event.events e ON e.id = m.event_id
            WHERE m.id = $1
              AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| ReunionFrancophone {
        title_fr: l.title_fr,
        title_en: l.title_en,
        status: l.status,
        jour: l.jour,
        debut: l.debut,
        fin: l.fin,
        format: l.format,
        lieu: l.venue_label,
        motif: l.cancellation_reason,
        ville: l.city,
        fuseau: l.fuseau,
        aujourdhui: l.aujourdhui,
    }))
}

/// Inscrites et liste d'attente (`meeting_audience()`).
pub async fn audience_reunion(conn: &mut PgConnection, id: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT p AS "p!" FROM negotiation.meeting_audience($1) AS p ORDER BY p"#,
        id
    )
    .fetch_all(conn)
    .await?)
}

pub async fn dans_laudience(conn: &mut PgConnection, id: Uuid, person: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT $2 IN (SELECT negotiation.meeting_audience($1)) AS "dedans!""#,
        id,
        person
    )
    .fetch_one(conn)
    .await?)
}

pub async fn inscrite(conn: &mut PgConnection, id: Uuid, person: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM negotiation.meeting_registrations
                           WHERE meeting_id = $1 AND person_id = $2 AND status = 'registered')
                  AS "inscrite!""#,
        id,
        person
    )
    .fetch_one(conn)
    .await?)
}
