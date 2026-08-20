//! La fiche telle que le contrat du front l'attend, et les deux issues de sa
//! création.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{MembershipId, OrganizationId, PersonId};
use super::search::SimilarOrganization;

/// `org.organization_status`. Machine à états **fermée** : un énuméré est
/// légitime, contrairement au type d'organisation qui vit dans
/// `reference.taxonomy_terms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationStatus {
    Candidate,
    Active,
    Merged,
    Archived,
    Rejected,
}

impl OrganizationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Active => "active",
            Self::Merged => "merged",
            Self::Archived => "archived",
            Self::Rejected => "rejected",
        }
    }

    /// L'énuméré est fermé en base : une valeur inconnue signale que le code et
    /// le modèle ont divergé, pas qu'un utilisateur a mal saisi quelque chose.
    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "candidate" => Some(Self::Candidate),
            "active" => Some(Self::Active),
            "merged" => Some(Self::Merged),
            "archived" => Some(Self::Archived),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// `Organization` de `frontend/app/types/org.ts` — la table, telle quelle.
#[derive(Debug, Clone, Serialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub legal_name: String,
    pub legal_name_normalized: Option<String>,
    pub acronym: Option<String>,
    pub acronym_normalized: Option<String>,
    pub slug: String,
    pub organization_type_code: String,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub description: Option<Value>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub status: OrganizationStatus,
    pub merged_into_id: Option<OrganizationId>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub merged_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub verified_by: Option<PersonId>,
    pub trust_score: i16,
    pub created_by: Option<PersonId>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Ce que le formulaire de création envoie — les sept champs du contrat, plus
/// la fonction déclarée et les fiches proches montrées.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrganization {
    pub legal_name: String,
    #[serde(default)]
    pub acronym: Option<String>,
    pub organization_type_code: String,
    #[serde(default)]
    pub country_id: Option<Uuid>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub description: Option<Value>,
    #[serde(default)]
    pub job_title: Option<String>,
    /// Fiches proches **affichées** avant la création, et maintenues malgré
    /// tout. Vide quand la recherche n'a rien ramené : créer sans rien voir
    /// n'est pas la même faute que créer en sachant.
    #[serde(default)]
    pub acknowledged_match_ids: Vec<Uuid>,
}

/// `CreateOrganizationResult`. Deux issues, et **aucune ne dit « refusé parce
/// qu'un doublon existe »** : la base ne refuse que le doublon exact, et le
/// refus porte alors la fiche en cause — de quoi la rejoindre.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreateOrganizationOutcome {
    Created {
        organization: Box<Organization>,
        membership_id: MembershipId,
        role: super::membership::MembershipRole,
    },
    NameTaken {
        existing: Box<SimilarOrganization>,
    },
}

/// Compose l'adresse d'URL d'une fiche à partir de son nom.
///
/// **Le repli n'est pas décoratif** : `platform.slugify` rend `NULL` quand la
/// normalisation efface tout le nom — un nom entièrement composé de signes de
/// ponctuation ou d'idéogrammes. La colonne est `NOT NULL` et le domaine
/// `platform.slug` refuse la chaîne vide : sans repli, la création échouerait en
/// erreur interne sur un nom que la base accepte par ailleurs.
pub fn slug_ou_repli(slug: Option<String>, id: Uuid) -> String {
    match slug {
        Some(s) if !s.is_empty() => s,
        _ => format!("org-{}", &id.simple().to_string()[..12]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nom_que_la_normalisation_efface_recoit_tout_de_meme_une_adresse() {
        let id = Uuid::now_v7();
        let repli = slug_ou_repli(None, id);
        assert!(repli.starts_with("org-"));
        assert_eq!(repli.len(), 16);

        assert_eq!(slug_ou_repli(Some(String::new()), id), repli);
        assert_eq!(slug_ou_repli(Some("ifdd".to_owned()), id), "ifdd");
    }
}
