//! Les termes proposés au lexique. Formes de `frontend/app/types/negotiation-savoir.ts`
//! (téléphone) et `admin-negotiation-queue.ts` (file). Aucune forme de la file
//! ne porte d'auteur ni l'expert qui tranche (R9).

use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::savoir::{verifier_longueur, KnowledgeStatus};

/// Par personne et par jour de Paris (R8).
pub const PLAFOND_PROPOSITIONS: i64 = 10;
/// `ck_glossary_proposals_term`.
pub const TERME_MAX: usize = 200;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalInput {
    pub client_ref: Uuid,
    pub term: String,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalReceipt {
    pub id: Uuid,
    pub client_ref: Uuid,
    pub term: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Ce que devient un terme proposé. Le terme déjà au lexique n'est pas une
/// `ApiError` : la réponse porte le `slug` de l'entrée, que l'erreur ne sait
/// pas porter.
#[derive(Debug, Clone)]
pub enum IssueProposition {
    Nouvelle(ProposalReceipt),
    Rejouee(ProposalReceipt),
    DejaAuLexique { slug: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminProposalContext {
    pub context: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminProposalNearby {
    pub id: Uuid,
    pub slug: String,
    pub term: String,
    pub status: KnowledgeStatus,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminProposal {
    pub id: Uuid,
    pub term: String,
    pub status: String,
    pub authors_count: i64,
    pub contexts: Vec<AdminProposalContext>,
    pub nearby: Vec<AdminProposalNearby>,
    pub glossary_entry_id: Option<Uuid>,
    pub glossary_entry_slug: Option<String>,
    pub rejection_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub handled_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminProposalRejectInput {
    pub reason: String,
}

/// Le terme et le contexte, tels qu'ils seront gardés.
pub fn proposition_valide(e: &ProposalInput) -> Result<(String, Option<String>)> {
    let term = e.term.trim();
    if term.is_empty() {
        return Err(ApiError::validation("Écrivez le terme à proposer.", "term"));
    }
    if term.chars().count() > TERME_MAX {
        return Err(ApiError::validation(
            "Un terme tient en 200 caractères au plus.",
            "term",
        ));
    }
    let context = e
        .context
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    if let Some(c) = context {
        verifier_longueur(c, "context")?;
    }
    Ok((term.to_owned(), context.map(str::to_owned)))
}

pub fn motif_valide(e: &AdminProposalRejectInput) -> Result<String> {
    let reason = e.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::validation(
            "Indiquez le motif du refus.",
            "reason",
        ));
    }
    Ok(reason.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::error::ErrorCode;

    fn proposition(term: &str, context: Option<&str>) -> ProposalInput {
        ProposalInput {
            client_ref: Uuid::now_v7(),
            term: term.into(),
            context: context.map(Into::into),
        }
    }

    #[test]
    fn terme_et_contexte_sont_rognes_et_bornes() {
        let (t, c) = proposition_valide(&proposition("  bracketed text ", Some("  "))).unwrap();
        assert_eq!((t.as_str(), c), ("bracketed text", None));
        assert!(proposition_valide(&proposition("   ", None)).is_err());
        assert!(proposition_valide(&proposition(&"a".repeat(201), None)).is_err());
        assert!(proposition_valide(&proposition(&"é".repeat(200), None)).is_ok());
        let long = proposition_valide(&proposition("terme", Some(&"é".repeat(601)))).unwrap_err();
        assert_eq!(long.code, ErrorCode::NegotiationTextTooLong);
        assert_eq!(long.field.as_deref(), Some("context"));
        assert!(motif_valide(&AdminProposalRejectInput { reason: " ".into() }).is_err());
    }
}
