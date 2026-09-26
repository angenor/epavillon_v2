//! La file des signalements et les décisions (research R3). « Valider » ne rend
//! rien public : seule la publication, trente secondes plus tard, le fait.

use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::domain::reports::{MyReport, DETAIL_MAX};

#[derive(Debug, Clone, Serialize)]
pub struct ReportAuthor {
    pub name: String,
    pub country: Option<String>,
}

/// Ce que dit la source à l'instant, avec son heure de lecture (ADR-010).
#[derive(Debug, Clone, Serialize)]
pub struct SourceNow {
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_at: Option<OffsetDateTime>,
    pub venue: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub read_at: Option<OffsetDateTime>,
}

/// `ReportQueueItem` — le statut est celui de la base : l'administration voit
/// « validé » dès la validation, avant la publication.
#[derive(Debug, Clone, Serialize)]
pub struct ReportQueueItem {
    #[serde(flatten)]
    pub report: MyReport,
    pub author: ReportAuthor,
    pub source_now: Option<SourceNow>,
    pub decided_by: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub withdrawn_at: Option<OffsetDateTime>,
}

/// `ReportQueue` — à traiter, les plus anciens d'abord ; puis les traités du
/// jour, dans le fuseau de la COP.
#[derive(Debug, Clone, Serialize)]
pub struct ReportQueue {
    pub pending: Vec<ReportQueueItem>,
    pub decided_today: Vec<ReportQueueItem>,
}

pub const MOTIFS_DE_REFUS: [&str; 3] = ["source_maintains", "already_known", "not_precise"];

/// `RejectPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct RejectPayload {
    pub reason: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug)]
pub struct Refus {
    pub reason: &'static str,
    pub detail: Option<String>,
}

impl RejectPayload {
    pub fn valider(&self) -> Result<Refus> {
        let reason = MOTIFS_DE_REFUS
            .into_iter()
            .find(|m| *m == self.reason.trim())
            .ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::NegotiationReportInvalid,
                    "Choisissez un motif parmi les trois proposés.",
                )
                .field("reason")
            })?;
        let detail = self
            .detail
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_owned);
        if detail
            .as_deref()
            .is_some_and(|d| d.chars().count() > DETAIL_MAX)
        {
            return Err(ApiError::with_message(
                ErrorCode::NegotiationReportInvalid,
                "La précision ne dépasse pas 600 caractères.",
            )
            .field("detail"));
        }
        Ok(Refus { reason, detail })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditionQuery {
    pub edition: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_refus_exige_un_des_trois_motifs() {
        let charge = |reason: &str| RejectPayload {
            reason: reason.into(),
            detail: Some("  ".into()),
        };
        let r = charge("already_known").valider().expect("valide");
        assert_eq!((r.reason, r.detail), ("already_known", None));
        assert_eq!(
            charge("autre").valider().unwrap_err().field.as_deref(),
            Some("reason")
        );
    }
}
