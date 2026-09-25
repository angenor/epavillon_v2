//! Trancher un signalement. Valider pose la publication et ne rend rien
//! public ; annuler gagne tant que rien n'est publié ; refuser prévient
//! l'autrice ; retirer ôte un encart affiché (research R3).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin_reports::{RejectPayload, ReportQueue, ReportQueueItem};
use crate::jobs::publish;
use crate::notifications::emission;
use crate::repo::admin_reports::{self as depot, Retrait};
use crate::repo::{publication, sessions};
use crate::state::NegotiationState;

pub async fn file(state: &NegotiationState, edition: &str, locale: &str) -> Result<ReportQueue> {
    let mut conn = state.pool().acquire().await?;
    let edition = sessions::edition(&mut conn, edition.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;
    let (mut pending, mut decided_today) = (Vec::new(), Vec::new());
    for lu in depot::lister(&mut conn, Some(edition.event_id), None, locale).await? {
        if lu.item.report.decided_at.is_none() {
            pending.push(lu.item);
        } else if lu.du_jour {
            decided_today.push(lu.item);
        }
    }
    decided_today.sort_by_key(|i| std::cmp::Reverse(i.report.decided_at));
    Ok(ReportQueue {
        pending,
        decided_today,
    })
}

async fn un(conn: &mut PgConnection, id: Uuid, locale: &str) -> Result<ReportQueueItem> {
    depot::lister(conn, None, Some(id), locale)
        .await?
        .into_iter()
        .next()
        .map(|lu| lu.item)
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationReportUnknown))
}

fn source_du_moment(item: &ReportQueueItem) -> Option<serde_json::Value> {
    item.source_now
        .as_ref()
        .and_then(|s| serde_json::to_value(s).ok())
}

/// Tient la ligne ; inconnue → 404, déjà tranchée → 409.
async fn a_trancher(conn: &mut PgConnection, id: Uuid, locale: &str) -> Result<ReportQueueItem> {
    let statut = depot::verrouiller(conn, id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationReportUnknown))?;
    if statut != "submitted" {
        return Err(ApiError::new(ErrorCode::NegotiationReportAlreadyDecided));
    }
    un(conn, id, locale).await
}

pub async fn valider(
    state: &NegotiationState,
    ctx: &RequestContext,
    decideur: Uuid,
    id: Uuid,
) -> Result<ReportQueueItem> {
    let mut tx = state.db().write(ctx).await?;
    let avant = a_trancher(&mut tx, id, &ctx.locale).await?;
    let validation = depot::valider(&mut tx, id, decideur, source_du_moment(&avant))
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationReportAlreadyDecided))?;
    publish::poser(&mut tx, id, validation.decided_at, validation.publier_a).await?;
    let apres = un(&mut tx, id, &ctx.locale).await?;
    tx.commit().await?;
    Ok(apres)
}

/// Aucune borne de temps : tant que rien n'est publié, l'annulation gagne. Une
/// seconde annulation rend l'état sans rien écrire.
pub async fn annuler(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<ReportQueueItem> {
    let mut tx = state.db().write(ctx).await?;
    if !depot::annuler(&mut tx, id).await? {
        let item = un(&mut tx, id, &ctx.locale).await?;
        if item.report.decided_at.is_some() {
            return Err(ApiError::new(ErrorCode::NegotiationReportUndoExpired));
        }
    }
    let item = un(&mut tx, id, &ctx.locale).await?;
    tx.commit().await?;
    Ok(item)
}

pub async fn refuser(
    state: &NegotiationState,
    ctx: &RequestContext,
    decideur: Uuid,
    id: Uuid,
    charge: &RejectPayload,
) -> Result<ReportQueueItem> {
    let refus = charge.valider()?;
    let mut tx = state.db().write(ctx).await?;
    let avant = a_trancher(&mut tx, id, &ctx.locale).await?;
    if !depot::refuser(&mut tx, id, decideur, &refus, source_du_moment(&avant)).await? {
        return Err(ApiError::new(ErrorCode::NegotiationReportAlreadyDecided));
    }
    let tenu = publication::tenir(&mut tx, id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationReportUnknown))?;
    emission::decision(&mut tx, &tenu).await?;
    let apres = un(&mut tx, id, &ctx.locale).await?;
    tx.commit().await?;
    Ok(apres)
}

pub async fn retirer(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<ReportQueueItem> {
    let mut tx = state.db().write(ctx).await?;
    depot::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationReportUnknown))?;
    match depot::retirer(&mut tx, id).await? {
        Retrait::Fait | Retrait::DejaRetire => {}
        Retrait::PasAffiche => {
            return Err(ApiError::with_message(
                ErrorCode::NegotiationReportAlreadyDecided,
                "Ce signalement n'est pas affiché : il n'y a rien à retirer.",
            ))
        }
    }
    let item = un(&mut tx, id, &ctx.locale).await?;
    tx.commit().await?;
    Ok(item)
}
