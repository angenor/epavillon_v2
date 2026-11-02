//! Ce qui attend encore : travaux de `platform.jobs` et événements de
//! `platform.outbox_events` en échec, lus en direct, et leur relance. Les
//! charges utiles ne sortent jamais : certaines portent un jeton en clair.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::entries::nettoyer;
use super::{conflit, JournalRead};
use crate::routes::platform::MaintenanceManage;

const LISTE_MAX: i64 = 200;

#[derive(Debug, Serialize)]
pub struct JournalOutboxEvent {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_schema: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub available_at: OffsetDateTime,
    pub attempts: i16,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JournalFailedJob {
    pub id: Uuid,
    pub queue: String,
    pub task: String,
    pub state: &'static str,
    pub attempts: i16,
    pub max_attempts: i16,
    pub last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_failed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub run_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct JournalFailures {
    pub jobs: Vec<JournalFailedJob>,
    pub events: Vec<JournalOutboxEvent>,
}

/// Travaux à relancer, événements non relayés après au moins un essai. Même
/// prédicat que les listes ci-dessous.
pub(super) async fn compter(pool: &PgPool) -> Result<(i64, i64)> {
    let c = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM platform.jobs j
                    WHERE j.status IN ('failed', 'dead')
                       OR (j.status = 'queued' AND j.attempts > 0 AND j.last_error IS NOT NULL)
                  ) AS "travaux!",
                  (SELECT count(*) FROM platform.outbox_events e
                    WHERE e.published_at IS NULL AND e.attempts > 0
                  ) AS "evenements!""#
    )
    .fetch_one(pool)
    .await?;
    Ok((c.travaux, c.evenements))
}

pub(super) async fn evenements_de_la_demande(
    pool: &PgPool,
    request_id: &str,
) -> Result<Vec<JournalOutboxEvent>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_type, aggregate_schema, aggregate_type, aggregate_id, occurred_at,
                  published_at, available_at, attempts, last_error
             FROM platform.outbox_events
            WHERE correlation_id = $1
            ORDER BY occurred_at, id
            LIMIT $2"#,
        request_id,
        LISTE_MAX
    )
    .fetch_all(pool)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| JournalOutboxEvent {
            id: l.id,
            event_type: l.event_type,
            aggregate_schema: l.aggregate_schema,
            aggregate_type: l.aggregate_type,
            aggregate_id: l.aggregate_id,
            occurred_at: l.occurred_at,
            published_at: l.published_at,
            available_at: l.available_at,
            attempts: l.attempts,
            last_error: nettoyer(l.last_error),
        })
        .collect())
}

fn etat(statut: &str) -> &'static str {
    match statut {
        "dead" => "dead",
        "failed" => "failed",
        _ => "retrying",
    }
}

#[utoipa::path(
    get,
    description = "`JournalFailures` : travaux morts, en échec ou remis en file après un échec (`state` : dead, failed, retrying), et événements de l'outbox non relayés après au moins un essai. 200 de chaque au plus, sans charge utile ; dernière erreur nettoyée des adresses et des jetons.",
    path = "/admin/journal/failures",
    tag = "Journal",
    operation_id = "admin_journal_echecs",
    responses(
        (status = 200, description = "JournalFailures", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn echecs(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
) -> Result<HttpResponse> {
    let travaux = sqlx::query!(
        r#"SELECT j.id, j.queue, j.task, j.status::text AS "status!", j.attempts, j.max_attempts,
                  j.last_error, j.run_at, j.created_at,
                  (SELECT max(o.occurred_at) FROM platform.ops_log o
                    WHERE o.kind = 'job_failed' AND o.job_id = j.id) AS last_failed_at
             FROM platform.jobs j
            WHERE j.status IN ('failed', 'dead')
               OR (j.status = 'queued' AND j.attempts > 0 AND j.last_error IS NOT NULL)
            ORDER BY j.run_at DESC, j.id DESC
            LIMIT $1"#,
        LISTE_MAX
    )
    .fetch_all(db.pool())
    .await?;

    let evenements = sqlx::query!(
        r#"SELECT id, event_type, aggregate_schema, aggregate_type, aggregate_id, occurred_at,
                  published_at, available_at, attempts, last_error
             FROM platform.outbox_events
            WHERE published_at IS NULL AND attempts > 0
            ORDER BY available_at, id
            LIMIT $1"#,
        LISTE_MAX
    )
    .fetch_all(db.pool())
    .await?;

    Ok(HttpResponse::Ok().json(JournalFailures {
        jobs: travaux
            .into_iter()
            .map(|l| JournalFailedJob {
                id: l.id,
                queue: l.queue,
                task: l.task,
                state: etat(&l.status),
                attempts: l.attempts,
                max_attempts: l.max_attempts,
                last_error: nettoyer(l.last_error),
                last_failed_at: l.last_failed_at,
                run_at: l.run_at,
                created_at: l.created_at,
            })
            .collect(),
        events: evenements
            .into_iter()
            .map(|l| JournalOutboxEvent {
                id: l.id,
                event_type: l.event_type,
                aggregate_schema: l.aggregate_schema,
                aggregate_type: l.aggregate_type,
                aggregate_id: l.aggregate_id,
                occurred_at: l.occurred_at,
                published_at: l.published_at,
                available_at: l.available_at,
                attempts: l.attempts,
                last_error: nettoyer(l.last_error),
            })
            .collect(),
    }))
}

fn contexte(requete: &HttpRequest) -> Result<RequestContext> {
    requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .ok_or_else(ApiError::unauthenticated)
}

#[utoipa::path(
    post,
    description = "Relance un travail mort ou en échec par `platform.retry_job()` : un essai de plus, tout de suite, compteur et dernière erreur gardés. Exige aussi `identity.platform.maintenance`.",
    path = "/admin/journal/jobs/{id}/retry",
    tag = "Journal",
    operation_id = "admin_journal_relancer_travail",
    params(("id" = Uuid, Path, description = "Identifiant du travail")),
    responses(
        (status = 204, description = "Remis en file"),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` ou `identity.platform.maintenance` absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 404, description = "Travail inconnu", body = crate::openapi::ApiErrorBody),
        (status = 409, description = "Le travail n'est ni mort ni en échec", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relancer_travail(
    _lire: Requires<JournalRead>,
    _relancer: Requires<MaintenanceManage>,
    db: web::Data<Db>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let mut tx = db.write(&contexte(&requete)?).await?;
    let relance = sqlx::query_scalar!(r#"SELECT platform.retry_job($1) AS "relance!""#, id)
        .fetch_one(&mut *tx)
        .await?;
    if !relance {
        let existe = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM platform.jobs WHERE id = $1) AS "existe!""#,
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        return Err(if existe {
            conflit("Ce travail n'est plus en échec : il a été relancé ou a réussi entre-temps.")
        } else {
            ApiError::not_found()
        });
    }
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "Rend un événement de l'outbox non relayé disponible tout de suite et réveille le relais. Exige aussi `identity.platform.maintenance`.",
    path = "/admin/journal/events/{id}/retry",
    tag = "Journal",
    operation_id = "admin_journal_relancer_evenement",
    params(("id" = Uuid, Path, description = "Identifiant de l'événement")),
    responses(
        (status = 204, description = "Disponible pour le relais"),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` ou `identity.platform.maintenance` absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 404, description = "Événement inconnu", body = crate::openapi::ApiErrorBody),
        (status = 409, description = "Événement déjà relayé", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relancer_evenement(
    _lire: Requires<JournalRead>,
    _relancer: Requires<MaintenanceManage>,
    db: web::Data<Db>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let mut tx = db.write(&contexte(&requete)?).await?;
    let touches = sqlx::query!(
        "UPDATE platform.outbox_events SET available_at = now()
          WHERE id = $1 AND published_at IS NULL",
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if touches == 0 {
        let existe = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM platform.outbox_events WHERE id = $1) AS "existe!""#,
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        return Err(if existe {
            conflit("Cet événement a déjà été relayé.")
        } else {
            ApiError::not_found()
        });
    }
    // Le relais écoute ce canal : sans réveil, il attendrait son prochain tour.
    sqlx::query!(
        "SELECT pg_notify('platform_outbox', $1::text)",
        id.to_string()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
