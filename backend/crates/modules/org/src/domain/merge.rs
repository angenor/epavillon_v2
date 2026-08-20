//! La fusion : les dix champs comparés, les six avertissements, les trois sorts
//! de transfert.
//!
//! **C'est l'opération la plus dangereuse du module : rien ne l'annule d'un
//! clic.** La fiche absorbée survit, mais ses rattachements sont déplacés et il
//! faudrait les reprendre un à un.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::duplicates::DuplicateSide;
use super::ids::OrganizationId;

/// Les dix champs comparés côte à côte : les colonnes qu'un humain a saisies.
/// Les colonnes engendrées, les états de fusion et les compteurs n'y figurent
/// pas — ils ne se choisissent pas.
pub const MERGE_FIELDS: &[&str] = &[
    "legal_name",
    "acronym",
    "slug",
    "organization_type_code",
    "country_id",
    "city",
    "description",
    "website",
    "contact_email",
    "contact_phone",
];

/// **Le champ que l'on compare mais qui ne se déplace pas.**
///
/// Contrairement au nom, l'unicité de l'adresse d'URL ne connaît **aucune
/// condition de statut** : la fiche absorbée garde la sienne pour toujours,
/// puisqu'elle survit. Trois issues, deux mauvaises — libérer l'adresse de la
/// source casse la promesse même de la fusion ; l'échanger fait mener l'ancienne
/// adresse de la survivante vers la fiche absorbée, ce qui est pire qu'une
/// adresse morte : c'est une adresse qui ment. Reste le refus, informatif, et
/// qui n'arrive que si l'opérateur choisit délibérément celle de la source
/// (research.md § R6).
pub const CHAMP_NON_ARBITRABLE: &str = "slug";

/// Quel côté fournit la valeur retenue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeSide {
    Source,
    Target,
}

/// Un champ, ses deux valeurs et leur écart.
///
/// **`differs` ne suffit pas à décider**, et c'est pourquoi `filled` existe :
/// les deux fiches portent une valeur différente — quelqu'un doit trancher —, ou
/// une seule est renseignée — il n'y a rien à trancher, on prend celle qui
/// existe. Confondre les deux imposerait douze arbitrages là où il n'y en a que
/// trois.
#[derive(Debug, Clone, Serialize)]
pub struct MergeFieldComparison {
    pub field: String,
    pub source_value: Value,
    pub target_value: Value,
    pub differs: bool,
    pub filled: &'static str,
    pub source_label: Option<Value>,
    pub target_label: Option<Value>,
}

impl MergeFieldComparison {
    pub fn nouvelle(field: &str, source: Value, cible: Value) -> Self {
        let source_pleine = !source.is_null() && source != Value::String(String::new());
        let cible_pleine = !cible.is_null() && cible != Value::String(String::new());

        Self {
            field: field.to_owned(),
            differs: source != cible,
            filled: match (source_pleine, cible_pleine) {
                (false, false) => "none",
                (true, false) => "source",
                (false, true) => "target",
                (true, true) => "both",
            },
            source_value: source,
            target_value: cible,
            source_label: None,
            target_label: None,
        }
    }
}

/// Une ligne du décompte de transfert — une entrée du registre
/// `org.organization_references`, chiffrée sur la paire courante.
///
/// **Trois sorts, pas un**, et les confondre ferait mentir le décompte :
/// `reassign` bascule la ligne, `delete` la supprime — quotas, politiques : ce
/// sont des réglages, pas un patrimoine —, et le dédoublonnage supprime une
/// ligne `reassign` dont la valeur existe déjà côté cible, sans quoi l'unicité
/// ferait échouer la fusion.
#[derive(Debug, Clone, Serialize)]
pub struct MergeTransferLine {
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_column: String,
    pub strategy: String,
    pub dedupe_on: Vec<String>,
    /// Lignes qui basculeront vers la fiche absorbante.
    pub reassigned: i64,
    /// Lignes **supprimées avant la bascule** parce que la cible porte déjà la
    /// même valeur. Ce ne sont pas des pertes : le rattachement existe déjà de
    /// l'autre côté.
    pub deduped: i64,
    /// Lignes supprimées par la stratégie `delete`.
    pub deleted: i64,
}

/// Ce qui doit **arrêter la main avant le geste**. Non bloquant : la fusion
/// reste possible, l'écran ne décide pas à la place de l'équipe.
#[derive(Debug, Clone, Serialize)]
pub struct MergeWarning {
    pub code: &'static str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
}

impl MergeWarning {
    pub fn simple(code: &'static str) -> Self {
        Self {
            code,
            values: BTreeMap::new(),
        }
    }

    pub fn avec(code: &'static str, cles: &[(&str, String)]) -> Self {
        Self {
            code,
            values: cles
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        }
    }
}

/// Les six avertissements, dans l'ordre où ils comptent.
///
/// **Le premier est le plus important** : absorber une fiche portant le sceau de
/// l'IFDD dans une fiche qui ne l'a pas fait perdre la vérification, et personne
/// ne s'en aperçoit avant qu'un public la cherche.
pub const AVERT_SOURCE_VERIFIEE: &str = "source_is_verified";
pub const AVERT_SOURCE_PLUS_ACTIVE: &str = "source_has_more_activity";
pub const AVERT_SOURCE_DOMAINE_VERIFIE: &str = "source_has_verified_domain";
pub const AVERT_CIBLE_NON_VERIFIEE: &str = "target_not_verified";
pub const AVERT_PAYS_DIFFERENTS: &str = "different_countries";
pub const AVERT_TYPES_DIFFERENTS: &str = "different_types";

/// Une dénomination que la source apportera à la cible. C'est ce qui fait qu'une
/// recherche sur l'ancien nom continue de trouver la bonne fiche.
#[derive(Debug, Clone, Serialize)]
pub struct TransferredName {
    pub name: String,
    pub kind: String,
    pub is_confirmed: bool,
    pub already_present: bool,
}

/// Un domaine apporté, avec son état de vérification.
#[derive(Debug, Clone, Serialize)]
pub struct TransferredDomain {
    pub domain: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<time::OffsetDateTime>,
    pub already_present: bool,
}

/// **Tout l'écran de fusion, pour un sens donné.**
///
/// Il est calculé pour un sens — source absorbée par cible — et **recalculé
/// quand on inverse** : le décompte n'est pas symétrique. Trois adhésions
/// transférées dans un sens peuvent en faire cinq dans l'autre, selon ce que
/// chaque fiche porte déjà.
#[derive(Debug, Clone, Serialize)]
pub struct MergePreview {
    /// La fiche **absorbée** : elle passera en statut `merged`.
    pub source: Box<DuplicateSide>,
    /// La fiche **absorbante** : elle survit, et c'est elle qu'on complète.
    pub target: Box<DuplicateSide>,
    pub pair_id: Option<Uuid>,
    pub comparisons: Vec<MergeFieldComparison>,
    pub transfers: Vec<MergeTransferLine>,
    pub transferred_names: Vec<TransferredName>,
    pub transferred_domains: Vec<TransferredDomain>,
    pub warnings: Vec<MergeWarning>,
}

/// La demande de fusion.
///
/// `confirmation_name` est le **nom de la fiche absorbée**, saisi à la main.
/// C'est le dernier verrou avant une opération que rien n'annule d'un clic —
/// et l'API le revérifie : masquer un bouton n'a jamais empêché une requête.
#[derive(Debug, Clone, Deserialize)]
pub struct MergePayload {
    pub source_id: Uuid,
    pub target_id: Uuid,
    #[serde(default)]
    pub pair_id: Option<Uuid>,
    /// Obligatoire — c'est ce qu'on relira dans six mois.
    pub reason: String,
    /// Pour chaque champ divergent, le côté dont la valeur est conservée. Absent
    /// du dictionnaire, le champ garde la valeur de la **cible** : c'est elle
    /// qui survit, et l'absence de choix ne doit rien écraser.
    #[serde(default)]
    pub field_choices: BTreeMap<String, MergeSide>,
    pub confirmation_name: String,
}

/// Ce que rend la fusion.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MergeOutcome {
    Merged {
        target: OrganizationId,
        /// Décompte **réel** des lignes déplacées, relu dans le journal.
        rows_reassigned: Value,
        /// Champs de la cible modifiés par les choix de l'opérateur.
        fields_applied: Vec<String>,
    },
    ConfirmationMismatch,
    /// Le message du trigger de la base, **repris mot pour mot**.
    AlreadyMerged {
        target: Option<OrganizationId>,
        message: String,
    },
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn une_valeur_absente_dun_cote_ne_demande_aucun_arbitrage() {
        let seule = MergeFieldComparison::nouvelle("city", json!("Dakar"), Value::Null);
        assert!(seule.differs);
        assert_eq!(seule.filled, "source");

        let deux = MergeFieldComparison::nouvelle("city", json!("Dakar"), json!("Ouagadougou"));
        assert_eq!(deux.filled, "both", "là, quelqu'un doit trancher");

        let aucune = MergeFieldComparison::nouvelle("city", Value::Null, Value::Null);
        assert!(!aucune.differs);
        assert_eq!(aucune.filled, "none");
    }

    #[test]
    fn une_chaine_vide_ne_compte_pas_comme_une_valeur() {
        let vide = MergeFieldComparison::nouvelle("city", json!(""), json!("Dakar"));
        assert_eq!(vide.filled, "target");
    }
}
