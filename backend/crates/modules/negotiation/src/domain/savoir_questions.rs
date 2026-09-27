//! Les questions aux experts. Formes de `frontend/app/types/negotiation-savoir.ts`
//! (téléphone) et `admin-negotiation-queue.ts` (file). Aucune forme de la file ne
//! porte l'auteure ni l'expert qui répond (R9).

use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::savoir::verifier_longueur;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MyQuestionInput {
    pub client_ref: Uuid,
    pub theme_code: String,
    pub body: String,
    pub consent_to_faq: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MyQuestion {
    pub id: Uuid,
    pub client_ref: Uuid,
    pub theme_code: String,
    pub theme_label: String,
    pub body: String,
    pub consent_to_faq: bool,
    pub status: String,
    pub answer: Option<String>,
    pub answered_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub answered_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct MyQuestionList {
    pub questions: Vec<MyQuestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminQuestion {
    pub id: Uuid,
    pub theme_code: String,
    pub theme_label: String,
    pub body: String,
    pub consent_to_faq: bool,
    pub status: String,
    pub answer: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub answered_at: Option<OffsetDateTime>,
    pub faq_entry_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminQuestionAnswerInput {
    pub answer: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminQuestionPromoteInput {
    pub section_code: String,
}

/// Le texte de la question, tel qu'il sera gardé.
pub fn question_valide(e: &MyQuestionInput) -> Result<String> {
    let body = e.body.trim();
    if body.is_empty() {
        return Err(ApiError::validation("Écrivez votre question.", "body"));
    }
    verifier_longueur(body, "body")?;
    Ok(body.to_owned())
}

pub fn reponse_valide(e: &AdminQuestionAnswerInput) -> Result<String> {
    let answer = e.answer.trim();
    if answer.is_empty() {
        return Err(ApiError::validation("Écrivez la réponse.", "answer"));
    }
    Ok(answer.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::error::ErrorCode;

    fn question(body: &str) -> MyQuestionInput {
        MyQuestionInput {
            client_ref: Uuid::now_v7(),
            theme_code: "adaptation".into(),
            body: body.into(),
            consent_to_faq: true,
        }
    }

    #[test]
    fn une_question_vide_ou_trop_longue_est_refusee() {
        assert_eq!(question_valide(&question("  Qui ?  ")).unwrap(), "Qui ?");
        assert!(question_valide(&question("   ")).is_err());
        let longue = question_valide(&question(&"é".repeat(601))).unwrap_err();
        assert_eq!(longue.code, ErrorCode::NegotiationTextTooLong);
        assert!(reponse_valide(&AdminQuestionAnswerInput { answer: " ".into() }).is_err());
    }
}
