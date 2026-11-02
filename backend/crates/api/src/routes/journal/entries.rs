//! Les lignes de `platform.ops_log` : chiffres de l'en-tête, liste filtrée,
//! export CSV, et la recherche par code de la demande, qui y joint l'audit et
//! l'outbox de la même requête.

use actix_web::http::header;
use actix_web::{web, HttpResponse};
use kernel::auth::Requires;
use kernel::db::Db;
use kernel::error::{ErrorCode, Result};
use kernel::ops_log::{sanitize_detail, OpsLog};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    changes, debut_de_retention, failures, invalide, motif, pagination, recherche, JournalPerson,
    JournalRead, Page, Period, RETENTION_MOIS,
};

const KINDS: [&str; 4] = ["http_error", "http_refused", "http_slow", "job_failed"];
const SOURCES: [&str; 2] = ["api", "worker"];
const CSV_MAX: i64 = 5000;
const TRACE_MAX: i64 = 500;

#[derive(Debug, Serialize)]
pub struct JournalEntry {
    pub id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub source: String,
    pub kind: String,
    pub level: String,
    pub request_id: Option<String>,
    pub actor: Option<JournalPerson>,
    pub method: Option<String>,
    pub route: Option<String>,
    pub status: Option<i16>,
    pub duration_ms: Option<i32>,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub task: Option<String>,
    pub job_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EntryQuery {
    pub period: Option<Period>,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub q: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

struct Filtre {
    depuis: OffsetDateTime,
    kind: Option<String>,
    source: Option<String>,
    q: Option<String>,
    request_id: Option<String>,
    limite: i64,
    decalage: i64,
}

impl EntryQuery {
    fn filtre(&self, limite: i64, decalage: i64) -> Result<Filtre> {
        let kind = self.kind.clone().filter(|k| !k.is_empty());
        if kind.as_deref().is_some_and(|k| !KINDS.contains(&k)) {
            return Err(invalide("kind"));
        }
        let source = self.source.clone().filter(|s| !s.is_empty());
        if source.as_deref().is_some_and(|s| !SOURCES.contains(&s)) {
            return Err(invalide("source"));
        }
        Ok(Filtre {
            depuis: self.period.unwrap_or_default().debut(),
            kind,
            source,
            q: recherche(self.q.as_deref()),
            request_id: None,
            limite,
            decalage,
        })
    }
}

/// Message du catalogue pour un refus ; détail nettoyé pour une panne.
fn message(detail: Option<String>, code: Option<&str>) -> Option<String> {
    detail.or_else(|| {
        code.and_then(|c| ErrorCode::ALL.iter().find(|e| e.as_str() == c))
            .map(|e| e.message().to_owned())
    })
}

async fn lire(pool: &PgPool, f: &Filtre) -> Result<(Vec<JournalEntry>, i64)> {
    let q_motif = f.q.as_deref().map(motif);
    let lignes = sqlx::query!(
        r#"WITH page AS (
               SELECT o.id, o.occurred_at, o.source, o.kind, o.level, o.request_id,
                      o.actor_id, p.display_name AS actor_name, o.method, o.route, o.status,
                      o.duration_ms, o.error_code, o.task, o.job_id, o.detail,
                      count(*) OVER () AS total
                 FROM platform.ops_log o
                 LEFT JOIN identity.people p ON p.id = o.actor_id
                WHERE o.occurred_at >= $1
                  AND ($2::text IS NULL OR o.kind = $2)
                  AND ($3::text IS NULL OR o.source = $3)
                  AND ($4::text IS NULL
                       OR o.request_id = $4
                       OR o.route ILIKE '%' || $5 || '%'
                       OR o.error_code ILIKE '%' || $5 || '%'
                       OR o.task ILIKE '%' || $5 || '%'
                       OR p.display_name ILIKE '%' || $5 || '%')
                  AND ($6::text IS NULL OR o.request_id = $6)
                ORDER BY o.occurred_at DESC, o.id DESC
                LIMIT $7 OFFSET $8
           )
           SELECT page.id AS "id!", page.occurred_at AS "occurred_at!", page.source AS "source!",
                  page.kind AS "kind!", page.level AS "level!", page.request_id AS "request_id?",
                  page.actor_id AS "actor_id?", page.actor_name AS "actor_name?",
                  g.nom AS "organization_name?", page.method AS "method?", page.route AS "route?",
                  page.status AS "status?", page.duration_ms AS "duration_ms?",
                  page.error_code AS "error_code?", page.task AS "task?", page.job_id AS "job_id?",
                  page.detail AS "detail?", page.total AS "total!"
             FROM page
             LEFT JOIN LATERAL (
                  SELECT COALESCE(o.acronym, o.legal_name) AS nom
                    FROM org.memberships m
                    JOIN org.organizations o ON o.id = m.organization_id
                   WHERE m.person_id = page.actor_id AND m.status = 'active'
                   ORDER BY m.is_primary DESC, m.approved_at NULLS LAST
                   LIMIT 1
             ) g ON true
            ORDER BY page.occurred_at DESC, page.id DESC"#,
        f.depuis,
        f.kind,
        f.source,
        f.q,
        q_motif,
        f.request_id,
        f.limite,
        f.decalage
    )
    .fetch_all(pool)
    .await?;

    let total = lignes.first().map_or(0, |l| l.total);
    let entrees = lignes
        .into_iter()
        .map(|l| JournalEntry {
            message: message(l.detail, l.error_code.as_deref()),
            actor: l.actor_id.map(|id| JournalPerson {
                id,
                display_name: l.actor_name.unwrap_or_default(),
                organization_name: l.organization_name,
            }),
            id: l.id,
            occurred_at: l.occurred_at,
            source: l.source,
            kind: l.kind,
            level: l.level,
            request_id: l.request_id,
            method: l.method,
            route: l.route,
            status: l.status,
            duration_ms: l.duration_ms,
            error_code: l.error_code,
            task: l.task,
            job_id: l.job_id,
        })
        .collect();
    Ok((entrees, total))
}

#[derive(Debug, Serialize)]
pub struct JournalSummary {
    #[serde(with = "time::serde::rfc3339")]
    pub since: OffsetDateTime,
    pub http_error: i64,
    pub http_refused: i64,
    pub http_slow: i64,
    pub job_failed: i64,
    pub failing_jobs: i64,
    pub failing_events: i64,
    pub dropped: u64,
    pub retention_months: u8,
}

#[utoipa::path(
    get,
    description = "`JournalSummary` : les quatre chiffres des 24 dernières heures (`platform.ops_log`), les travaux et événements en échec à relancer, et les lignes perdues par ce processus depuis son démarrage.",
    path = "/admin/journal/summary",
    tag = "Journal",
    operation_id = "admin_journal_resume",
    responses(
        (status = 200, description = "JournalSummary", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn resume(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
    journal: Option<web::Data<OpsLog>>,
) -> Result<HttpResponse> {
    let depuis = Period::Day.debut();
    let c = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE kind = 'http_error')   AS "http_error!",
                  count(*) FILTER (WHERE kind = 'http_refused') AS "http_refused!",
                  count(*) FILTER (WHERE kind = 'http_slow')    AS "http_slow!",
                  count(*) FILTER (WHERE kind = 'job_failed')   AS "job_failed!"
             FROM platform.ops_log
            WHERE occurred_at >= $1"#,
        depuis
    )
    .fetch_one(db.pool())
    .await?;
    let (failing_jobs, failing_events) = failures::compter(db.pool()).await?;

    Ok(HttpResponse::Ok().json(JournalSummary {
        since: depuis,
        http_error: c.http_error,
        http_refused: c.http_refused,
        http_slow: c.http_slow,
        job_failed: c.job_failed,
        failing_jobs,
        failing_events,
        dropped: journal.map_or(0, |j| j.dropped()),
        retention_months: RETENTION_MOIS,
    }))
}

#[utoipa::path(
    get,
    description = "`JournalEntryPage` : lignes de `platform.ops_log`, les plus récentes d'abord. Filtres : `period` (24h, 7d, 30d, 90d), `kind`, `source`, `q` (personne, route, code d'erreur ou code de la demande) ; `page` dès 1, `per_page` 50 par défaut, 200 au plus.",
    path = "/admin/journal/entries",
    tag = "Journal",
    operation_id = "admin_journal_entrees",
    params(
        ("period" = Option<String>, Query, description = "24h, 7d, 30d ou 90d"),
        ("kind" = Option<String>, Query, description = "http_error, http_refused, http_slow ou job_failed"),
        ("source" = Option<String>, Query, description = "api ou worker"),
        ("q" = Option<String>, Query, description = "Recherche libre"),
        ("page" = Option<i64>, Query, description = "Dès 1"),
        ("per_page" = Option<i64>, Query, description = "200 au plus"),
    ),
    responses(
        (status = 200, description = "JournalEntryPage", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 422, description = "Filtre inconnu", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn entrees(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
    requete: web::Query<EntryQuery>,
) -> Result<HttpResponse> {
    let (page, per_page, decalage) = pagination(requete.page, requete.per_page);
    let (items, total) = lire(db.pool(), &requete.filtre(per_page, decalage)?).await?;
    Ok(HttpResponse::Ok().json(Page {
        items,
        total,
        page,
        per_page,
    }))
}

#[utoipa::path(
    get,
    description = "Les lignes de `/admin/journal/entries` (mêmes filtres, 5 000 au plus) en CSV, séparateur virgule, UTF-8 avec marque d'ordre.",
    path = "/admin/journal/entries.csv",
    tag = "Journal",
    operation_id = "admin_journal_entrees_csv",
    params(
        ("period" = Option<String>, Query, description = "24h, 7d, 30d ou 90d"),
        ("kind" = Option<String>, Query, description = "http_error, http_refused, http_slow ou job_failed"),
        ("source" = Option<String>, Query, description = "api ou worker"),
        ("q" = Option<String>, Query, description = "Recherche libre"),
    ),
    responses(
        (status = 200, description = "Fichier CSV", body = String, content_type = "text/csv"),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 422, description = "Filtre inconnu", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn entrees_csv(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
    requete: web::Query<EntryQuery>,
) -> Result<HttpResponse> {
    let (entrees, _) = lire(db.pool(), &requete.filtre(CSV_MAX, 0)?).await?;
    let nom = format!(
        "journal-{}.csv",
        OffsetDateTime::now_utc()
            .format(time::macros::format_description!(
                "[year][month][day]-[hour][minute]"
            ))
            .unwrap_or_default()
    );
    Ok(HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{nom}\""),
        ))
        .body(csv(&entrees)))
}

fn csv(entrees: &[JournalEntry]) -> String {
    let mut sortie = String::from("\u{feff}");
    sortie.push_str("heure,nature,statut,methode,route,duree_ms,personne,organisation,code,message,tache,code_demande\r\n");
    for e in entrees {
        let date = e
            .occurred_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let champs = [
            date,
            e.kind.clone(),
            e.status.map(|s| s.to_string()).unwrap_or_default(),
            e.method.clone().unwrap_or_default(),
            e.route.clone().unwrap_or_default(),
            e.duration_ms.map(|d| d.to_string()).unwrap_or_default(),
            e.actor
                .as_ref()
                .map(|a| a.display_name.clone())
                .unwrap_or_default(),
            e.actor
                .as_ref()
                .and_then(|a| a.organization_name.clone())
                .unwrap_or_default(),
            e.error_code.clone().unwrap_or_default(),
            e.message.clone().unwrap_or_default(),
            e.task.clone().unwrap_or_default(),
            e.request_id.clone().unwrap_or_default(),
        ];
        let ligne: Vec<String> = champs.iter().map(|c| cellule(c)).collect();
        sortie.push_str(&ligne.join(","));
        sortie.push_str("\r\n");
    }
    sortie
}

/// Guillemets au besoin, et une cellule qui commencerait une formule de
/// tableur est neutralisée.
fn cellule(valeur: &str) -> String {
    let neutre = if valeur.starts_with(['=', '+', '-', '@']) {
        format!("'{valeur}")
    } else {
        valeur.to_owned()
    };
    if neutre.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", neutre.replace('"', "\"\""))
    } else {
        neutre
    }
}

#[derive(Debug, Serialize)]
pub struct JournalRequestTrace {
    pub request_id: String,
    pub entries: Vec<JournalEntry>,
    pub changes: Vec<changes::JournalChange>,
    pub events: Vec<failures::JournalOutboxEvent>,
}

#[utoipa::path(
    get,
    description = "`JournalRequestTrace` : tout ce qui porte ce code de demande, dans l'ordre chronologique : lignes du journal d'exploitation, modifications de l'audit (valeurs sensibles masquées par `platform.audit_is_sensitive`) et événements de l'outbox (`correlation_id`). Listes vides si rien ne correspond.",
    path = "/admin/journal/requests/{request_id}",
    tag = "Journal",
    operation_id = "admin_journal_demande",
    params(("request_id" = String, Path, description = "Valeur de l'en-tête X-Request-Id, ou `job:<uuid>` pour un travail")),
    responses(
        (status = 200, description = "JournalRequestTrace", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn demande(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
    chemin: web::Path<String>,
) -> Result<HttpResponse> {
    let request_id: String = chemin.into_inner().trim().chars().take(128).collect();
    let filtre = Filtre {
        depuis: debut_de_retention(),
        kind: None,
        source: None,
        q: None,
        request_id: Some(request_id.clone()),
        limite: TRACE_MAX,
        decalage: 0,
    };
    let (mut entries, _) = lire(db.pool(), &filtre).await?;
    entries.reverse();
    let mut modifs = changes::de_la_demande(db.pool(), &request_id, TRACE_MAX).await?;
    modifs.reverse();
    let events = failures::evenements_de_la_demande(db.pool(), &request_id).await?;

    Ok(HttpResponse::Ok().json(JournalRequestTrace {
        request_id,
        entries,
        changes: modifs,
        events,
    }))
}

/// Dernière erreur d'un travail ou d'un événement : même nettoyage qu'à
/// l'écriture du journal, elle n'y est pas passée.
pub(super) fn nettoyer(erreur: Option<String>) -> Option<String> {
    erreur.map(|e| sanitize_detail(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cellule_echappee_et_formule_neutralisee() {
        assert_eq!(cellule("simple"), "simple");
        assert_eq!(cellule("a,b"), "\"a,b\"");
        assert_eq!(cellule("dit \"non\""), "\"dit \"\"non\"\"\"");
        assert_eq!(cellule("=SOMME(A1)"), "'=SOMME(A1)");
    }

    #[test]
    fn message_du_catalogue_pour_un_refus() {
        assert_eq!(
            message(None, Some("FORBIDDEN")).as_deref(),
            Some(ErrorCode::Forbidden.message())
        );
        assert_eq!(
            message(Some("panne".into()), Some("INTERNAL")).as_deref(),
            Some("panne")
        );
        assert_eq!(message(None, Some("INCONNU")), None);
    }
}
