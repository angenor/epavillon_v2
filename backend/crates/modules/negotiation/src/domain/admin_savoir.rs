//! Les formes du back-office du savoir — FAQ, lexique, parcours. Elles suivent
//! `frontend/app/types/admin-negotiation-savoir.ts`. Aucune ne porte l'auteur
//! d'un retour ou d'un signalement (R9).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::admin_documents::champ;
use crate::domain::savoir::KnowledgeStatus;

#[derive(Debug, Clone, Serialize)]
pub struct AdminKnowledgeSource {
    pub document_id: Option<Uuid>,
    pub document_title: Option<String>,
    pub external_title: Option<String>,
    pub external_url: Option<String>,
    pub section_label: Option<String>,
    pub page_from: Option<i16>,
    pub page_to: Option<i16>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminKnowledgeSourceInput {
    pub document_id: Option<Uuid>,
    pub external_title: Option<String>,
    pub external_url: Option<String>,
    pub section_label: Option<String>,
    pub page_from: Option<i16>,
    pub page_to: Option<i16>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminKnowledgeRef {
    pub id: Uuid,
    pub label: String,
    pub status: KnowledgeStatus,
}

// --- FAQ -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AdminFaqList {
    pub entries: Vec<AdminFaqRow>,
    pub can_publish: bool,
    pub can_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminFaqRow {
    pub id: Uuid,
    pub section_code: String,
    pub question: String,
    pub status: KnowledgeStatus,
    pub verified_on: Option<Date>,
    pub has_answer: bool,
    pub open_reports: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub first_published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AdminFaqFeedback {
    pub helpful: i64,
    pub not_helpful: i64,
    pub too_vague: i64,
    pub off_topic: i64,
    pub outdated: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminFaqReport {
    pub id: Uuid,
    pub reasons: Vec<String>,
    pub from_feedback: bool,
    pub details: Option<String>,
    pub status: String,
    pub outcome: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub handled_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminFaqEntry {
    pub id: Uuid,
    pub section_code: String,
    pub question: Value,
    pub answer: Option<Value>,
    pub status: KnowledgeStatus,
    pub verified_on: Option<Date>,
    pub verified_by_name: Option<String>,
    pub editorial_rank: Option<i16>,
    pub origin_question_id: Option<Uuid>,
    pub sources: Vec<AdminKnowledgeSource>,
    pub related: Vec<AdminKnowledgeRef>,
    pub feedback: AdminFaqFeedback,
    pub reports: Vec<AdminFaqReport>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub first_published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub can_publish: bool,
    pub can_review: bool,
}

/// Un champ absent ne change rien ; `null` vide un champ facultatif. Sources et
/// liées se remplacent en bloc.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminFaqInput {
    pub section_code: Option<String>,
    pub question: Option<Value>,
    #[serde(default, deserialize_with = "champ")]
    pub answer: Option<Option<Value>>,
    #[serde(default, deserialize_with = "champ")]
    pub editorial_rank: Option<Option<i16>>,
    pub sources: Option<Vec<AdminKnowledgeSourceInput>>,
    pub related_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminFaqVerifyInput {
    pub verified_on: Option<Date>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FiltreSavoir {
    pub q: Option<String>,
    pub section: Option<String>,
    pub family: Option<String>,
    pub status: Option<String>,
}

// --- Lexique -------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AdminGlossaryList {
    pub entries: Vec<AdminGlossaryRow>,
    pub can_publish: bool,
    pub can_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminGlossaryRow {
    pub id: Uuid,
    pub slug: String,
    pub family_code: String,
    pub term: String,
    pub acronym: Option<String>,
    pub translation: String,
    pub status: KnowledgeStatus,
    #[serde(with = "time::serde::rfc3339::option")]
    pub first_published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminGlossaryEntry {
    pub id: Uuid,
    pub slug: String,
    pub family_code: String,
    pub term: String,
    pub acronym: Option<String>,
    pub variants: Vec<String>,
    pub translation: Value,
    pub definition: Value,
    pub heard_in_room: Option<String>,
    pub status: KnowledgeStatus,
    pub sources: Vec<AdminKnowledgeSource>,
    pub related: Vec<AdminKnowledgeRef>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub first_published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub can_publish: bool,
    pub can_review: bool,
}

/// Sans `slug` : il naît du terme et ne se recalcule jamais. Un `slug` envoyé
/// est ignoré.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminGlossaryInput {
    pub family_code: Option<String>,
    pub term: Option<String>,
    #[serde(default, deserialize_with = "champ")]
    pub acronym: Option<Option<String>>,
    pub variants: Option<Vec<String>>,
    pub translation: Option<Value>,
    pub definition: Option<Value>,
    #[serde(default, deserialize_with = "champ")]
    pub heard_in_room: Option<Option<String>>,
    pub sources: Option<Vec<AdminKnowledgeSourceInput>>,
    pub related_ids: Option<Vec<Uuid>>,
}

// --- Parcours ------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AdminPathwayLink {
    pub kind: String,
    pub target_id: Uuid,
    pub target_label: Option<String>,
    pub page: Option<i16>,
    pub section: Option<String>,
    pub label: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPathwayStep {
    pub id: Uuid,
    pub group_id: Uuid,
    pub label: Value,
    pub detail: Option<Value>,
    pub origin_label: Option<Value>,
    pub link: Option<AdminPathwayLink>,
    pub sort_order: i16,
    pub is_published: bool,
    pub checks: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPathwayGroup {
    pub id: Uuid,
    pub label: Value,
    pub sort_order: i16,
    pub is_published: bool,
    pub steps: Vec<AdminPathwayStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPathway {
    pub groups: Vec<AdminPathwayGroup>,
    pub can_publish: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminPathwayGroupInput {
    pub label: Option<Value>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminPathwayLinkInput {
    pub kind: String,
    pub target_id: Uuid,
    pub page: Option<i16>,
    pub section: Option<String>,
    pub label: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminPathwayStepInput {
    pub group_id: Option<Uuid>,
    pub label: Option<Value>,
    #[serde(default, deserialize_with = "champ")]
    pub detail: Option<Option<Value>>,
    #[serde(default, deserialize_with = "champ")]
    pub origin_label: Option<Option<Value>>,
    #[serde(default, deserialize_with = "champ")]
    pub link: Option<Option<AdminPathwayLinkInput>>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminPathwayOrderGroup {
    pub id: Uuid,
    pub step_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminPathwayOrderInput {
    pub groups: Vec<AdminPathwayOrderGroup>,
}

/// `draft`, `published` ou `to_review` ; autre chose ne filtre pas.
pub fn statut_filtre(brut: Option<&str>) -> Option<&'static str> {
    match brut? {
        "draft" => Some("draft"),
        "published" => Some("published"),
        "to_review" => Some("to_review"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_champ_absent_nest_pas_un_champ_vide() {
        let absent: AdminFaqInput = serde_json::from_str("{}").unwrap();
        assert!(absent.answer.is_none() && absent.sources.is_none());
        let vide: AdminFaqInput = serde_json::from_str(r#"{"answer": null}"#).unwrap();
        assert_eq!(vide.answer, Some(None));
        let lien: AdminPathwayStepInput = serde_json::from_str(r#"{"link": null}"#).unwrap();
        assert!(matches!(lien.link, Some(None)));
    }

    #[test]
    fn le_slug_envoye_est_ignore() {
        let e: AdminGlossaryInput =
            serde_json::from_str(r#"{"term": "contact group", "slug": "pirate"}"#).unwrap();
        assert_eq!(e.term.as_deref(), Some("contact group"));
    }
}
