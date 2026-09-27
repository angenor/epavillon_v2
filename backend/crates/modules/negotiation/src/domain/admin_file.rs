//! La file des experts. Formes de `frontend/app/types/admin-negotiation-queue.ts`.
//! Aucune ne porte d'auteur, ni l'expert qui clôt (R9).

use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::domain::admin_savoir::{AdminFaqFeedback, AdminFaqReport};
use crate::domain::savoir::KnowledgeStatus;
use crate::domain::savoir_propositions::AdminProposal;
use crate::domain::savoir_questions::AdminQuestion;

pub const ISSUES: [&str; 3] = ["revised", "confirmed", "dismissed"];

#[derive(Debug, Clone, Deserialize)]
pub struct FiltreFile {
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExpertQueueCounts {
    pub reports: i64,
    pub questions: i64,
    pub proposals: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpertQueueFaqRef {
    pub id: Uuid,
    pub question: String,
    pub status: KnowledgeStatus,
    pub verified_on: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpertQueueReportGroup {
    pub entry: ExpertQueueFaqRef,
    pub feedback: AdminFaqFeedback,
    pub reports: Vec<AdminFaqReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpertQueue {
    pub kind: &'static str,
    pub counts: ExpertQueueCounts,
    pub reports: Vec<ExpertQueueReportGroup>,
    pub questions: Vec<AdminQuestion>,
    pub proposals: Vec<AdminProposal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminFaqReportCloseInput {
    pub outcome: String,
}

/// Sans `kind`, les signalements.
pub fn sorte(brut: Option<&str>) -> Result<&'static str> {
    match brut.unwrap_or("reports") {
        "reports" => Ok("reports"),
        "questions" => Ok("questions"),
        "proposals" => Ok("proposals"),
        _ => Err(ApiError::validation(
            "Indiquez « reports », « questions » ou « proposals ».",
            "kind",
        )),
    }
}

pub fn issue(brut: &str) -> Result<&'static str> {
    ISSUES.iter().find(|i| **i == brut).copied().ok_or_else(|| {
        ApiError::validation(
            "Indiquez « revised », « confirmed » ou « dismissed ».",
            "outcome",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trois_issues_et_pas_une_de_plus() {
        for i in ISSUES {
            assert_eq!(issue(i).unwrap(), i);
        }
        assert!(issue("deleted").is_err());
        assert_eq!(sorte(None).unwrap(), "reports");
        assert_eq!(sorte(Some("questions")).unwrap(), "questions");
        assert_eq!(sorte(Some("proposals")).unwrap(), "proposals");
        assert!(sorte(Some("everything")).is_err());
    }
}
