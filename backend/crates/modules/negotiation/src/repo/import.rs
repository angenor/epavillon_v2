//! L'écriture de l'import de la source officielle : réglage, état, sessions,
//! écarts, compteurs et journal. Une transaction par lecture, ouverte par le
//! travail sous son contexte : aucune de ces écritures n'émet d'événement.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use std::collections::HashMap;
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::import::comparaison::{Changement, EnBase, Etat, Fiche, Motif};
use crate::import::denominations::{TermeLu, Vocabulaires};

pub struct Reglage {
    pub id: Uuid,
    pub event_id: Uuid,
    pub is_enabled: bool,
    pub reader: String,
    pub archive_name: Option<String>,
    pub archive_first_day: Option<Date>,
    pub live_url: Option<String>,
    pub time_correction_minutes: i16,
    pub interval_seconds: i32,
    pub edition_slug: String,
    pub timezone: String,
}

pub async fn reglage(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<Reglage>> {
    let ligne = sqlx::query!(
        r#"SELECT i.id, i.event_id, i.is_enabled, i.reader, i.archive_name, i.archive_first_day,
                  i.live_url::text AS live_url, i.time_correction_minutes, i.interval_seconds,
                  e.slug::text AS "slug!", e.timezone::text AS "timezone!"
             FROM negotiation.official_imports i
             JOIN event.events e ON e.id = i.event_id
            WHERE i.event_id = $1"#,
        event_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| Reglage {
        id: l.id,
        event_id: l.event_id,
        is_enabled: l.is_enabled,
        reader: l.reader,
        archive_name: l.archive_name,
        archive_first_day: l.archive_first_day,
        live_url: l.live_url,
        time_correction_minutes: l.time_correction_minutes,
        interval_seconds: l.interval_seconds,
        edition_slug: l.slug,
        timezone: l.timezone,
    }))
}

/// Les imports allumés et leur intervalle : ce que le worker réarme.
pub async fn allumes(conn: &mut PgConnection) -> Result<Vec<(Uuid, i32)>> {
    let lignes = sqlx::query!(
        "SELECT event_id, interval_seconds FROM negotiation.official_imports WHERE is_enabled"
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| (l.event_id, l.interval_seconds))
        .collect())
}

pub async fn vocabulaires(conn: &mut PgConnection) -> Result<Vocabulaires> {
    let lignes = sqlx::query!(
        r#"SELECT id, taxonomy_code, code, metadata
             FROM reference.taxonomy_terms
            WHERE taxonomy_code IN ('negotiation_meeting_type', 'negotiation_group') AND is_active
            ORDER BY sort_order, code"#
    )
    .fetch_all(conn)
    .await?;

    let mut types = Vec::new();
    let mut groupes = Vec::new();
    for l in lignes {
        let terme = TermeLu {
            id: l.id,
            code: l.code,
            metadata: l.metadata,
        };
        if l.taxonomy_code == "negotiation_meeting_type" {
            types.push(terme);
        } else {
            groupes.push(terme);
        }
    }
    Ok(Vocabulaires::depuis(types, groupes))
}

/// Situe des heures murales dans le fuseau de l'édition. PostgreSQL porte la
/// base des fuseaux : le code n'en embarque pas une seconde.
pub async fn situer(
    conn: &mut PgConnection,
    heures: &[PrimitiveDateTime],
    fuseau: &str,
) -> Result<Vec<OffsetDateTime>> {
    let instants = sqlx::query_scalar!(
        r#"SELECT (h AT TIME ZONE $2) AS "instant!"
             FROM unnest($1::timestamp[]) WITH ORDINALITY AS u(h, n)
            ORDER BY n"#,
        heures,
        fuseau
    )
    .fetch_all(conn)
    .await?;
    Ok(instants)
}

/// Les sessions importées de l'édition, et le groupe de chacune.
pub async fn etat(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<(Vec<EnBase>, HashMap<Uuid, Option<Uuid>>)> {
    let lignes = sqlx::query!(
        r#"SELECT m.id AS "id!", m.source_key AS "source_key!", m.start_at AS "start_at!", m.end_at, m.venue_label,
                  m.title_original AS "title_original!", t.code AS "type_code?",
                  m.is_open_access, a.code AS "point?", m.status::text AS "status!",
                  m.cancellation_reason, m.absent_reads AS "absent_reads!", m.group_term_id
             FROM negotiation.meetings m
             LEFT JOIN reference.taxonomy_terms t ON t.id = m.meeting_type_term_id
             LEFT JOIN negotiation.agenda_items a ON a.id = m.agenda_item_id
            WHERE m.event_id = $1 AND m.source_key IS NOT NULL"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    let groupes = lignes.iter().map(|l| (l.id, l.group_term_id)).collect();
    let sessions = lignes
        .into_iter()
        .map(|l| EnBase {
            id: l.id,
            cle: l.source_key,
            absences: l.absent_reads,
            fiche: Fiche {
                debut: l.start_at,
                fin: l.end_at,
                salle: l.venue_label,
                titre: l.title_original,
                type_code: l.type_code.unwrap_or_default(),
                acces_ouvert: l.is_open_access,
                point: l.point,
                etat: if l.status == "cancelled" {
                    Etat::Annulee(
                        l.cancellation_reason
                            .as_deref()
                            .and_then(Motif::depuis_code)
                            .unwrap_or(Motif::Source),
                    )
                } else {
                    Etat::Prevue
                },
            },
        })
        .collect();
    Ok((sessions, groupes))
}

pub async fn espace_climat(conn: &mut PgConnection) -> Result<Uuid> {
    let id = sqlx::query_scalar!("SELECT id FROM negotiation.spaces WHERE slug = 'climat'")
        .fetch_optional(conn)
        .await?;
    id.ok_or_else(|| kernel::error::ApiError::internal("espace de négociation « climat » absent"))
}

/// Les points de l'édition, par code ; ceux que la lecture cite pour la
/// première fois sont créés. Un point n'est jamais supprimé ni réécrit ici.
pub async fn points(
    conn: &mut PgConnection,
    event_id: Uuid,
    cites: &[(String, String)],
    lu_a: OffsetDateTime,
) -> Result<HashMap<String, Uuid>> {
    let mut connus: HashMap<String, Uuid> = sqlx::query!(
        "SELECT id, code FROM negotiation.agenda_items WHERE event_id = $1",
        event_id
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|l| (l.code, l.id))
    .collect();

    for (code, intitule) in cites {
        if connus.contains_key(code) {
            continue;
        }
        let id = sqlx::query_scalar!(
            "INSERT INTO negotiation.agenda_items (event_id, code, title, first_read_at)
             VALUES ($1, $2, $3, $4)
             RETURNING id",
            event_id,
            code,
            intitule,
            lu_a
        )
        .fetch_one(&mut *conn)
        .await?;
        connus.insert(code.clone(), id);
    }
    Ok(connus)
}

/// Ce qu'une lecture pose sur une session, apparue ou changée.
pub struct Ecriture<'a> {
    pub fiche: &'a Fiche,
    pub type_id: Uuid,
    pub groupe: Option<Uuid>,
    pub point: Option<Uuid>,
}

fn statut(etat: Etat) -> &'static str {
    match etat {
        Etat::Prevue => "scheduled",
        Etat::Annulee(_) => "cancelled",
    }
}

pub struct Origine<'a> {
    pub event_id: Uuid,
    pub space_id: Uuid,
    pub slug: &'a str,
    pub fuseau: &'a str,
    pub cle: &'a str,
    pub url: &'a str,
}

pub async fn inserer(
    conn: &mut PgConnection,
    origine: &Origine<'_>,
    e: &Ecriture<'_>,
    lu_a: OffsetDateTime,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO negotiation.meetings
               (space_id, kind, slug, title, start_at, end_at, timezone, format, venue_label,
                event_id, is_ifdd_organized, status, cancellation_reason, cancelled_at,
                source_key, source_url, title_original, first_read_at, last_read_at,
                meeting_type_term_id, group_term_id, agenda_item_id, is_open_access)
           VALUES ($1, 'negotiation_session', $2::text::platform.slug,
                   jsonb_build_object('fr', $3::text, 'en', $3::text)::platform.i18n_text,
                   $4, $5, $6::text::platform.timezone_name, 'onsite', $7,
                   $8, false, $9::text::negotiation.meeting_status, $10,
                   CASE WHEN $9::text = 'cancelled' THEN $11::timestamptz END,
                   $12, $13::text::platform.url, $3, $11, $11,
                   $14, $15, $16, $17)
           RETURNING id"#,
        origine.space_id,
        slug_de(origine.slug, origine.cle),
        e.fiche.titre,
        e.fiche.debut,
        e.fiche.fin,
        origine.fuseau,
        e.fiche.salle,
        origine.event_id,
        statut(e.fiche.etat),
        e.fiche.etat.motif(),
        lu_a,
        origine.cle,
        origine.url,
        e.type_id,
        e.groupe,
        e.point,
        e.fiche.acces_ouvert
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

fn slug_de(edition: &str, cle: &str) -> String {
    let cle: String = cle
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let cle = cle
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("{edition}-{cle}")
}

/// Changée ou reparue : tout est réécrit, les absences retombent à zéro.
/// L'heure du constat d'une annulation ne bouge pas tant qu'elle dure.
pub async fn modifier(
    conn: &mut PgConnection,
    id: Uuid,
    e: &Ecriture<'_>,
    lu_a: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE negotiation.meetings
              SET start_at = $2, end_at = $3, venue_label = $4,
                  title = jsonb_build_object('fr', $5::text, 'en', $5::text)::platform.i18n_text,
                  title_original = $5, meeting_type_term_id = $6, group_term_id = $7,
                  agenda_item_id = $8, is_open_access = $9,
                  status = $10::text::negotiation.meeting_status,
                  cancellation_reason = $11,
                  cancelled_at = CASE WHEN $10::text <> 'cancelled' THEN NULL
                                      WHEN status = 'cancelled' THEN cancelled_at
                                      ELSE $12::timestamptz END,
                  absent_reads = 0, last_read_at = $12
            WHERE id = $1"#,
        id,
        e.fiche.debut,
        e.fiche.fin,
        e.fiche.salle,
        e.fiche.titre,
        e.type_id,
        e.groupe,
        e.point,
        e.fiche.acces_ouvert,
        statut(e.fiche.etat),
        e.fiche.etat.motif(),
        lu_a
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Absente d'une lecture réussie ; `annuler` à la deuxième de suite.
pub async fn noter_absence(
    conn: &mut PgConnection,
    id: Uuid,
    absences: i16,
    annuler: bool,
    lu_a: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE negotiation.meetings
              SET absent_reads = $2,
                  status = CASE WHEN $3 THEN 'cancelled'::negotiation.meeting_status ELSE status END,
                  cancellation_reason = CASE WHEN $3 THEN 'removed' ELSE cancellation_reason END,
                  cancelled_at = CASE WHEN $3 THEN $4 ELSE cancelled_at END
            WHERE id = $1"#,
        id,
        absences,
        annuler,
        lu_a
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn noter_changements(
    conn: &mut PgConnection,
    id: Uuid,
    changements: &[Changement],
    lu_a: OffsetDateTime,
    run_id: Uuid,
) -> Result<()> {
    for c in changements {
        sqlx::query!(
            "INSERT INTO negotiation.meeting_changes
                 (meeting_id, field, old_value, new_value, detected_at, import_run_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
            id,
            c.champ,
            c.avant,
            c.apres,
            lu_a,
            run_id
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Les sessions lues sans écart : leur dernière lecture, en une requête.
pub async fn marquer_lues(
    conn: &mut PgConnection,
    ids: &[Uuid],
    lu_a: OffsetDateTime,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "UPDATE negotiation.meetings SET last_read_at = $2 WHERE id = ANY($1)",
        ids,
        lu_a
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Le groupe re-résolu, sans écart ni changement : une dénomination ajoutée
/// au vocabulaire rattache les coordinations déjà importées.
pub async fn rattacher_groupes(
    conn: &mut PgConnection,
    rattachements: &[(Uuid, Option<Uuid>)],
) -> Result<()> {
    if rattachements.is_empty() {
        return Ok(());
    }
    let (ids, groupes): (Vec<Uuid>, Vec<Option<Uuid>>) = rattachements.iter().copied().unzip();
    sqlx::query!(
        "UPDATE negotiation.meetings m SET group_term_id = u.groupe
           FROM unnest($1::uuid[], $2::uuid[]) AS u(id, groupe)
          WHERE m.id = u.id",
        &ids,
        &groupes as &[Option<Uuid>]
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub struct Lecture {
    pub import_id: Uuid,
    pub debut: OffsetDateTime,
    pub manuelle: bool,
}

/// Le journal d'une lecture, posé avant les écarts qui le citent.
pub async fn journaliser(
    conn: &mut PgConnection,
    lecture: &Lecture,
    erreur: Option<&str>,
    sessions: Option<i32>,
    ecarts: Option<i32>,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        "INSERT INTO negotiation.import_runs
             (import_id, started_at, finished_at, outcome, error, session_count, change_count, is_manual)
         VALUES ($1, $2, now(), $3, $4, $5, $6, $7)
         RETURNING id",
        lecture.import_id,
        lecture.debut,
        if erreur.is_some() { "failure" } else { "success" },
        erreur,
        sessions,
        ecarts,
        lecture.manuelle
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

pub async fn noter_reussite(
    conn: &mut PgConnection,
    import_id: Uuid,
    ecarts: i32,
    lu_a: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.official_imports
            SET missed_reads = 0, failing_since = NULL, last_error = NULL,
                last_success_at = $2, last_attempt_at = $2, last_change_count = $3
          WHERE id = $1",
        import_id,
        lu_a,
        ecarts
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn noter_echec(
    conn: &mut PgConnection,
    import_id: Uuid,
    erreur: &str,
    a: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.official_imports
            SET missed_reads = LEAST(missed_reads + 1, 32767),
                failing_since = COALESCE(failing_since, $2),
                last_error = $3, last_attempt_at = $2
          WHERE id = $1",
        import_id,
        a,
        erreur
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn purger_journal(conn: &mut PgConnection, import_id: Uuid) -> Result<u64> {
    let supprimes = sqlx::query!(
        "DELETE FROM negotiation.import_runs
          WHERE import_id = $1 AND started_at < now() - interval '30 days'",
        import_id
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(supprimes)
}

/// Les titres de l'édition qu'aucune traduction ne couvre encore — ce que le
/// travail de traduction (phase 3, T023) prendra.
pub async fn titres_sans_traduction(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<String>> {
    let titres = sqlx::query_scalar!(
        r#"SELECT DISTINCT m.title_original AS "titre!"
             FROM negotiation.meetings m
            WHERE m.event_id = $1 AND m.source_key IS NOT NULL
              AND NOT EXISTS (SELECT 1 FROM negotiation.title_translations t
                               WHERE t.source_text = m.title_original)"#,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(titres)
}
