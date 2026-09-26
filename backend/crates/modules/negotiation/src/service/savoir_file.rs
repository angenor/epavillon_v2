//! La file des experts. Clore un signalement ne touche jamais l'entrée : la
//! corriger ou la mettre « À revoir » passe par sa fiche (FR-018).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::admin_file::{
    issue, sorte, AdminFaqReportCloseInput, ExpertQueue, ExpertQueueFaqRef, ExpertQueueReportGroup,
};
use crate::domain::admin_savoir::AdminFaqReport;
use crate::domain::savoir::KnowledgeStatus;
use crate::repo::savoir_faq as faq;
use crate::repo::savoir_file as repo;
use crate::service::{savoir_propositions, savoir_questions};
use crate::state::NegotiationState;

pub async fn file(
    state: &NegotiationState,
    kind: Option<&str>,
    locale: &str,
) -> Result<ExpertQueue> {
    let kind = sorte(kind)?;
    let mut conn = state.pool().acquire().await?;
    let counts = repo::comptes(&mut conn).await?;
    let mut reports: Vec<ExpertQueueReportGroup> = Vec::new();
    if kind != "reports" {
        drop(conn);
        let (questions, proposals) = if kind == "questions" {
            (savoir_questions::file(state, locale).await?, Vec::new())
        } else {
            (Vec::new(), savoir_propositions::file(state).await?)
        };
        return Ok(ExpertQueue {
            kind,
            counts,
            reports,
            questions,
            proposals,
        });
    }
    for o in repo::ouverts(&mut conn, locale).await? {
        match reports.iter_mut().find(|g| g.entry.id == o.entry_id) {
            Some(groupe) => groupe.reports.push(o.report),
            None => reports.push(ExpertQueueReportGroup {
                entry: ExpertQueueFaqRef {
                    id: o.entry_id,
                    question: o.question,
                    status: KnowledgeStatus::depuis(&o.status),
                    verified_on: o.verified_on,
                },
                feedback: faq::retours(&mut conn, o.entry_id).await?,
                reports: vec![o.report],
            }),
        }
    }
    Ok(ExpertQueue {
        kind,
        counts,
        reports,
        questions: Vec::new(),
        proposals: Vec::new(),
    })
}

pub async fn clore_un_signalement(
    state: &NegotiationState,
    ctx: &RequestContext,
    expert: Uuid,
    id: Uuid,
    entree: &AdminFaqReportCloseInput,
) -> Result<AdminFaqReport> {
    let issue = issue(&entree.outcome)?;
    let mut tx = state.db().write(ctx).await?;
    match repo::statut(&mut tx, id).await?.as_deref() {
        None => return Err(ApiError::not_found()),
        Some("open") => {}
        Some(_) => return Err(ApiError::new(ErrorCode::NegotiationQueueItemClosed)),
    }
    let clos = repo::clore(&mut tx, id, issue, expert).await?;
    tx.commit().await?;
    Ok(clos)
}
