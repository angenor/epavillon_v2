//! Les formes du savoir — FAQ, parcours, lexique. Elles suivent
//! `frontend/app/types/negotiation-savoir.ts`, leur source unique.

use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

/// Longueur maximale d'un texte libre du lecteur : détail d'un signalement,
/// question, contexte d'une proposition.
pub const TEXTE_MAX: usize = 600;

/// Une transaction ouverte avant `served_at` et validée après porte un
/// `updated_at` antérieur : la différence remonte d'autant (R2).
pub const CHEVAUCHEMENT: Duration = Duration::minutes(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Draft,
    Published,
    ToReview,
}

impl KnowledgeStatus {
    /// La valeur de l'ENUM, lue en texte.
    pub fn depuis(valeur: &str) -> Self {
        match valeur {
            "published" => Self::Published,
            "to_review" => Self::ToReview,
            _ => Self::Draft,
        }
    }

    /// `to_review` reste servi, avec sa mention (tranché le 25/09).
    pub fn servi(self) -> bool {
        !matches!(self, Self::Draft)
    }
}

/// `KnowledgeBundle` — tout le publié (`complete`), ou ce qui a changé depuis
/// `since` avec les identifiants sortis.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeBundle {
    #[serde(with = "time::serde::rfc3339")]
    pub served_at: OffsetDateTime,
    pub complete: bool,
    pub faq_sections: Vec<FaqSection>,
    pub glossary_families: Vec<GlossaryFamily>,
    pub faq: Vec<FaqEntry>,
    pub glossary: Vec<GlossaryEntry>,
    pub pathway: Pathway,
    pub most_read: Vec<Uuid>,
    pub removed: Removed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaqSection {
    pub code: String,
    pub label: String,
    pub icon: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlossaryFamily {
    pub code: String,
    pub label: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaqEntry {
    pub id: Uuid,
    pub section_code: String,
    pub question: String,
    pub answer: String,
    pub status: KnowledgeStatus,
    pub verified_on: Date,
    pub sources: Vec<KnowledgeSource>,
    /// Tous les liens, publiés ou non : le téléphone filtre sur ce qu'il a.
    pub related_ids: Vec<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlossaryEntry {
    pub id: Uuid,
    pub slug: String,
    pub family_code: String,
    pub term: String,
    pub acronym: Option<String>,
    pub variants: Vec<String>,
    pub translation: String,
    pub definition: String,
    pub heard_in_room: Option<String>,
    pub sources: Vec<KnowledgeSource>,
    pub related_ids: Vec<Uuid>,
    pub status: KnowledgeStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Un document de la bibliothèque, titre joint pour la lecture sans elle, ou
/// une référence extérieure.
#[derive(Debug, Clone, Default, Serialize)]
pub struct KnowledgeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_from: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_to: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Pathway {
    pub groups: Vec<PathwayGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathwayGroup {
    pub id: Uuid,
    pub label: String,
    pub sort_order: i32,
    pub steps: Vec<PathwayStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathwayStep {
    pub id: Uuid,
    pub label: String,
    pub detail: Option<String>,
    pub origin_label: Option<String>,
    pub link: Option<PathwayLink>,
    pub sort_order: i32,
}

/// `kind` : `document`, `faq` ou `glossary` ; `target_id` désigne l'entrée ou
/// le document. Page et section ne valent que pour un document.
#[derive(Debug, Clone, Serialize)]
pub struct PathwayLink {
    pub kind: String,
    pub target_id: Uuid,
    pub page: Option<i16>,
    pub section: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Removed {
    pub faq: Vec<Uuid>,
    pub glossary: Vec<Uuid>,
}

/// `MyGlossaryFavorites` — les termes favoris de la personne connectée.
#[derive(Debug, Clone, Serialize)]
pub struct MyGlossaryFavorites {
    pub entry_ids: Vec<Uuid>,
}

/// `since` tel que le téléphone le renvoie : le `served_at` d'une lecture.
pub fn lire_since(brut: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(brut.trim(), &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::validation("Indiquez une date au format RFC 3339.", "since"))
}

/// Un texte libre du lecteur, compté en caractères.
pub fn verifier_longueur(texte: &str, champ: &str) -> Result<()> {
    if texte.chars().count() > TEXTE_MAX {
        return Err(ApiError::new(ErrorCode::NegotiationTextTooLong).field(champ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn since_se_lit_en_rfc3339_et_refuse_le_reste() {
        assert!(lire_since("2026-11-10T09:30:00Z").is_ok());
        assert!(lire_since("2026-11-10T10:30:00+01:00").is_ok());
        let refus = lire_since("hier").unwrap_err();
        assert_eq!(refus.code, ErrorCode::ValidationFailed);
        assert_eq!(refus.field.as_deref(), Some("since"));
    }

    #[test]
    fn six_cents_caracteres_passent_pas_un_de_plus() {
        let juste = "é".repeat(TEXTE_MAX);
        assert!(verifier_longueur(&juste, "details").is_ok());
        let trop = format!("{juste}a");
        assert_eq!(
            verifier_longueur(&trop, "details").unwrap_err().code,
            ErrorCode::NegotiationTextTooLong
        );
    }
}
