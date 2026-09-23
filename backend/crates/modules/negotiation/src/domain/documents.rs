//! Les formes des lectures publiques des documents. Elles suivent
//! `frontend/app/types/negotiation-documents.ts`, leur source unique.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// `DocumentLibrary` — la liste entière des documents publiés, et les libellés
/// des seules valeurs qu'elle cite.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentLibrary {
    pub documents: Vec<LibraryDocument>,
    pub vocabulary: LibraryVocabulary,
    /// Hors de l'empreinte : il change à chaque lecture.
    #[serde(with = "time::serde::rfc3339")]
    pub served_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryDocument {
    pub id: Uuid,
    pub slug: String,
    pub version: String,
    pub title: String,
    /// Nul pour un réservé sans accès.
    pub summary: Option<String>,
    #[serde(rename = "type")]
    pub type_code: String,
    pub themes: Vec<String>,
    /// Vrai pour tout réservé sans accès : qu'il en porte ou non ne se dit pas.
    pub themes_hidden: bool,
    pub cop: Option<Uuid>,
    pub issued_on: Option<Date>,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub publisher: Option<String>,
    pub locale: String,
    /// `file` ou `link`.
    pub source: &'static str,
    pub external_url: Option<String>,
    pub link_host: Option<String>,
    pub restricted: bool,
    pub accessible: bool,
    pub page_count: Option<i32>,
    pub reading_bytes: Option<i64>,
    /// `reflow` ou `as_is`, pour un fichier lisible.
    pub mode: Option<&'static str>,
    pub superseded_by: Option<Successor>,
    /// L'empreinte de la forme lisible : la copie gardée est-elle la bonne ?
    pub reading_etag: Option<String>,
}

/// Le bout publié de la chaîne de remplacement.
#[derive(Debug, Clone, Serialize)]
pub struct Successor {
    pub id: Uuid,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub page_count: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LibraryVocabulary {
    pub types: Vec<VocabularyTerm>,
    pub themes: Vec<VocabularyTerm>,
    pub cops: Vec<VocabularyCop>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VocabularyTerm {
    pub code: String,
    pub label: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VocabularyCop {
    pub id: Uuid,
    pub label: String,
    pub city: Option<String>,
}

/// `DocumentTextHits` — ce que la recherche dans le texte trouve. Pour un
/// réservé sans accès : l'identifiant seul, ni page ni extrait.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentTextHits {
    pub query: String,
    pub hits: Vec<TextHit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextHit {
    pub document_id: Uuid,
    pub pages: Vec<PageHit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageHit {
    pub index: i32,
    pub label: String,
    pub excerpt: String,
}

/// `DocumentReading` — la forme lisible entière, dans la grammaire de
/// `contracts/forme-lisible.md`.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentReading {
    pub id: Uuid,
    pub version: String,
    pub mode: &'static str,
    pub page_count: i32,
    pub outline: Value,
    pub pages: Vec<ReadingPage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingPage {
    pub index: i32,
    pub label: String,
    /// Le chemin d'API de l'image, relatif à la base : présent en « tel quel »,
    /// et sur les seules pages d'origine en mode recomposé.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub blocks: Value,
}

/// `CorrectionNoteList` — les notes vivantes des documents publiés.
#[derive(Debug, Clone, Serialize)]
pub struct CorrectionNoteList {
    pub notes: Vec<CorrectionNote>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorrectionNote {
    pub id: Uuid,
    pub document_id: Uuid,
    pub page_index: i32,
    pub passage: Option<String>,
    pub body: String,
    pub author_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub posted_at: OffsetDateTime,
}

/// `DocumentBookmarkList` — les favoris de la personne connectée.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentBookmarkList {
    pub bookmarks: Vec<DocumentBookmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentBookmark {
    pub document_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// L'hôte d'un lien, sans `www.` : « enb.iisd.org ».
pub fn hote(url: &str) -> Option<String> {
    let reste = url.split_once("://").map_or(url, |(_, r)| r);
    let hote = reste.split(['/', '?', '#']).next()?.rsplit('@').next()?;
    let hote = hote.split(':').next()?.trim_start_matches("www.");
    (!hote.is_empty()).then(|| hote.to_lowercase())
}

/// Le chemin public de l'image d'une page, que le téléphone garde tel quel.
pub fn chemin_image(document_id: Uuid, index: i32) -> String {
    format!("/negotiation/documents/{document_id}/pages/{index}/image")
}

/// L'empreinte de la forme lisible : figée tant que le fichier, son extraction
/// et le mode ne changent pas. Les notes n'y sont pas.
pub fn empreinte_de_lecture(
    document_id: Uuid,
    asset_id: Uuid,
    extrait_le: OffsetDateTime,
    tel_quel: bool,
) -> String {
    kernel::empreinte::de(&format!(
        "{document_id}:{asset_id}:{}:{tel_quel}",
        extrait_le.unix_timestamp_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lhote_dun_lien() {
        assert_eq!(
            hote("https://enb.iisd.org/cop30").as_deref(),
            Some("enb.iisd.org")
        );
        assert_eq!(
            hote("https://www.UNFCCC.int").as_deref(),
            Some("unfccc.int")
        );
        assert_eq!(
            hote("http://user@exemple.org:8080/x?y").as_deref(),
            Some("exemple.org")
        );
    }
}
