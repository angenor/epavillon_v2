//! « Cette réponse vous a-t-elle aidée ? » et « Dépassé ou faux ». Formes de
//! `frontend/app/types/negotiation-savoir.ts`.

use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::savoir::verifier_longueur;

/// Signalements par personne et par jour de Paris (R8).
pub const PLAFOND_SIGNALEMENTS: i64 = 20;

pub const MOTIFS_MANQUE: [&str; 3] = ["too_vague", "off_topic", "outdated"];
pub const MOTIFS_SIGNALEMENT: [&str; 3] = ["rule_changed", "wrong", "source_mismatch"];

/// Le motif qui part chez les experts.
pub const DEPASSEE: &str = "outdated";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaqFeedbackInput {
    pub helpful: bool,
    #[serde(default)]
    pub missing_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaqFeedback {
    pub entry_id: Uuid,
    pub helpful: bool,
    pub missing_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct MyFaqFeedback {
    pub feedback: Vec<FaqFeedback>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaqReportInput {
    pub client_ref: Uuid,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaqReportReceipt {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub client_ref: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Le motif après « Non », ou rien. « Oui » n'en porte pas.
pub fn motif_du_retour(e: &FaqFeedbackInput) -> Result<Option<&str>> {
    let Some(motif) = e.missing_reason.as_deref() else {
        return Ok(None);
    };
    if e.helpful || !MOTIFS_MANQUE.contains(&motif) {
        return Err(ApiError::validation(
            "Choisissez « Trop vague », « Ne répond pas à ma question » ou « Dépassée ou fausse », après « Non » seulement.",
            "missing_reason",
        ));
    }
    Ok(Some(motif))
}

/// Motifs dédoublonnés, dans l'ordre de la feuille ; précision rognée, vide
/// ramenée à rien.
pub fn signalement_valide(e: &FaqReportInput) -> Result<(Vec<String>, Option<String>)> {
    if let Some(inconnu) = e
        .reasons
        .iter()
        .find(|m| !MOTIFS_SIGNALEMENT.contains(&m.as_str()))
    {
        return Err(ApiError::validation(
            format!("Motif inconnu : {inconnu}."),
            "reasons",
        ));
    }
    let motifs: Vec<String> = MOTIFS_SIGNALEMENT
        .iter()
        .filter(|m| e.reasons.iter().any(|r| r == *m))
        .map(|m| (*m).to_owned())
        .collect();
    if motifs.is_empty() {
        return Err(ApiError::new(ErrorCode::NegotiationReportReasonRequired).field("reasons"));
    }
    let details = e
        .details
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty());
    if let Some(d) = details {
        verifier_longueur(d, "details")?;
    }
    Ok((motifs, details.map(str::to_owned)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retour(helpful: bool, motif: Option<&str>) -> FaqFeedbackInput {
        FaqFeedbackInput {
            helpful,
            missing_reason: motif.map(str::to_owned),
        }
    }

    fn signalement(motifs: &[&str], details: Option<&str>) -> FaqReportInput {
        FaqReportInput {
            client_ref: Uuid::nil(),
            reasons: motifs.iter().map(|m| (*m).to_owned()).collect(),
            details: details.map(str::to_owned),
        }
    }

    #[test]
    fn un_motif_ne_suit_que_non() {
        assert_eq!(motif_du_retour(&retour(true, None)).unwrap(), None);
        assert_eq!(
            motif_du_retour(&retour(false, Some("outdated"))).unwrap(),
            Some("outdated")
        );
        assert!(motif_du_retour(&retour(true, Some("too_vague"))).is_err());
        assert!(motif_du_retour(&retour(false, Some("boring"))).is_err());
    }

    #[test]
    fn un_signalement_exige_un_motif_connu() {
        assert_eq!(
            signalement_valide(&signalement(&[], None))
                .unwrap_err()
                .code,
            ErrorCode::NegotiationReportReasonRequired
        );
        assert_eq!(
            signalement_valide(&signalement(&["pirate"], None))
                .unwrap_err()
                .code,
            ErrorCode::ValidationFailed
        );
        let (motifs, details) = signalement_valide(&signalement(
            &["wrong", "rule_changed", "wrong"],
            Some("  "),
        ))
        .unwrap();
        assert_eq!(motifs, ["rule_changed", "wrong"]);
        assert_eq!(details, None);
    }
}
