//! Les formes du back-office des documents et des notes de correction. Elles
//! suivent `frontend/app/types/admin-negotiation-documents.ts`.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// `draft`, `published` ou `unpublished`.
pub fn etat(
    published_at: Option<OffsetDateTime>,
    unpublished_at: Option<OffsetDateTime>,
) -> &'static str {
    match (published_at, unpublished_at) {
        (Some(_), _) => "published",
        (None, Some(_)) => "unpublished",
        (None, None) => "draft",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentLink {
    pub id: Uuid,
    pub title: String,
    pub version: String,
}

/// L'état de l'extraction du fichier du moment.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractionState {
    /// `pending`, `extracting`, `ready` ou `failed`.
    pub status: String,
    pub page_count: Option<i32>,
    pub is_reflowable: Option<bool>,
    pub serve_as_is: bool,
    pub failure_reason: Option<String>,
    pub reading_bytes: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub extracted_at: Option<OffsetDateTime>,
}

/// `AdminDocumentList` — une ligne par document, et ce que la personne peut
/// faire.
#[derive(Debug, Clone, Serialize)]
pub struct AdminDocumentList {
    pub documents: Vec<AdminDocumentRow>,
    pub can_publish: bool,
    pub can_correct: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminDocumentRow {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "type")]
    pub type_code: String,
    pub version: String,
    pub state: &'static str,
    pub source: Option<&'static str>,
    pub restricted: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub supersedes: Option<DocumentLink>,
    pub superseded_by: Option<DocumentLink>,
    pub extraction: Option<ExtractionState>,
}

/// `AdminDocument` — la fiche d'un document, textes non résolus.
#[derive(Debug, Clone, Serialize)]
pub struct AdminDocument {
    pub id: Uuid,
    pub slug: String,
    pub title: Value,
    pub summary: Option<Value>,
    #[serde(rename = "type")]
    pub type_code: String,
    pub themes: Vec<String>,
    pub cop: Option<Uuid>,
    pub version: String,
    pub issued_on: Option<Date>,
    pub publisher: Option<String>,
    pub locale: String,
    pub source: Option<&'static str>,
    pub asset_id: Option<Uuid>,
    pub file: Option<AdminDocumentFile>,
    pub external_url: Option<String>,
    pub supersedes: Option<DocumentLink>,
    pub superseded_by: Option<DocumentLink>,
    pub restricted: bool,
    pub rag_eligible: bool,
    pub state: &'static str,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub unpublished_at: Option<OffsetDateTime>,
    /// Vrai : le fichier est figé, car le document a déjà été publié.
    pub file_locked: bool,
    pub extraction: Option<ExtractionState>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub can_publish: bool,
    pub can_correct: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminDocumentFile {
    pub filename: Option<String>,
    pub byte_size: i64,
    pub mime_type: String,
}

/// Un champ qui distingue « absent » (`None`) de « vidé » (`Some(None)`).
fn champ<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// `AdminDocumentInput` — un champ absent ne change rien ; `null` vide un champ
/// facultatif. À la création, `title` et `type` sont exigés.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminDocumentInput {
    pub title: Option<Value>,
    #[serde(default, deserialize_with = "champ")]
    pub summary: Option<Option<Value>>,
    #[serde(rename = "type")]
    pub type_code: Option<String>,
    pub themes: Option<Vec<String>>,
    #[serde(default, deserialize_with = "champ")]
    pub cop: Option<Option<Uuid>>,
    pub version: Option<String>,
    #[serde(default, deserialize_with = "champ")]
    pub issued_on: Option<Option<Date>>,
    #[serde(default, deserialize_with = "champ")]
    pub publisher: Option<Option<String>>,
    pub locale: Option<String>,
    #[serde(default, deserialize_with = "champ")]
    pub supersedes_id: Option<Option<Uuid>>,
    pub restricted: Option<bool>,
    pub rag_eligible: Option<bool>,
    #[serde(default, deserialize_with = "champ")]
    pub external_url: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachFileInput {
    pub asset_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServeAsIsInput {
    pub serve_as_is: bool,
}

/// `AdminDocumentPreview` — le verdict, le sommaire, et chaque page : ses blocs
/// et l'adresse de son image.
#[derive(Debug, Clone, Serialize)]
pub struct AdminDocumentPreview {
    pub id: Uuid,
    pub extraction: Option<ExtractionState>,
    pub quality: Option<Value>,
    pub extractor: Option<String>,
    pub outline: Value,
    pub pages: Vec<AdminPreviewPage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPreviewPage {
    pub index: i32,
    pub label: String,
    pub blocks: Value,
    pub image: Option<String>,
    pub has_origin_block: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonLink {
    pub id: Uuid,
    pub name: String,
}

/// `AdminCorrectionNote` — une note, vivante ou retirée, avec ses auteurs.
#[derive(Debug, Clone, Serialize)]
pub struct AdminCorrectionNote {
    pub id: Uuid,
    pub document_id: Uuid,
    pub page_index: i32,
    pub passage: Option<String>,
    pub body: Value,
    pub author: PersonLink,
    #[serde(with = "time::serde::rfc3339")]
    pub posted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub withdrawn_at: Option<OffsetDateTime>,
    pub withdrawn_by: Option<PersonLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCorrectionNoteList {
    pub notes: Vec<AdminCorrectionNote>,
    pub can_post: bool,
    pub can_withdraw: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorrectionNoteInput {
    pub page_index: i32,
    pub passage: Option<String>,
    pub body: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_champ_absent_nest_pas_un_champ_vide() {
        let absent: AdminDocumentInput = serde_json::from_str("{}").unwrap();
        assert!(absent.summary.is_none() && absent.cop.is_none());
        let vide: AdminDocumentInput =
            serde_json::from_str(r#"{"summary": null, "cop": null}"#).unwrap();
        assert_eq!(vide.summary, Some(None));
        assert_eq!(vide.cop, Some(None));
    }

    #[test]
    fn letat_dun_document() {
        let t = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(etat(None, None), "draft");
        assert_eq!(etat(Some(t), None), "published");
        assert_eq!(etat(None, Some(t)), "unpublished");
    }
}
