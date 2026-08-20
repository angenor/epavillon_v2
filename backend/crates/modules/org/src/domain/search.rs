//! Le résultat de recherche, ses bornes, et la marge de sur-lecture.
//!
//! La forme rendue est celle de `SimilarOrganization` (`frontend/app/types/org.ts`) :
//! les treize colonnes de `org.find_similar_organizations()`, telles quelles.
//! Le score n'est **pas** recalculé ici — il l'est déjà en base, et le refaire
//! en Rust ferait diverger l'API de l'écran sur un cas invisible.

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::OrganizationId;

/// Motif qui a fait entrer une fiche. Le front en connaît quatre valeurs ; on
/// n'en fige **pas** un énuméré — le jour où la fonction du modèle en ajoute
/// une, la désérialisation échouerait au lieu de la transmettre.
pub const MATCH_NAME_SIMILARITY: &str = "name_similarity";

/// Ce que la personne cherche doit faire au moins deux caractères.
///
/// **Sous le seuil, une liste vide et non une erreur** : le front ne le demande
/// jamais — son anti-rebond ne part qu'à deux signes —, et le garde existe pour
/// qu'un appel forgé ne balaie pas la table.
pub const MIN_TERM_LEN: usize = 2;

pub const DEFAULT_LIMIT: i32 = 10;
pub const MAX_LIMIT: i32 = 50;

/// De combien la lecture destinée à une personne sur-lit la fonction.
///
/// La limite est appliquée **à l'intérieur** de `find_similar_organizations()` :
/// filtrer après coup rendrait moins de lignes que demandé — dix demandées,
/// trois écartées, sept rendues. La marge reste petite et bornée parce que le
/// nombre de fiches entrant par le seul domaine est le nombre de fiches
/// déclarant le domaine de l'appelant : une, deux dans le cas des deux fiches
/// OSED (research.md § R1).
pub const OVERREAD_MARGIN: i32 = 5;

/// Borne la limite demandée. Un `limit=100000` forgé retombe au maximum, une
/// valeur absente ou absurde au défaut.
pub fn bounded_limit(demandee: Option<i32>) -> i32 {
    match demandee {
        Some(n) if n >= 1 => n.min(MAX_LIMIT),
        _ => DEFAULT_LIMIT,
    }
}

/// Une ligne de `org.find_similar_organizations()`, rendue telle quelle.
#[derive(Debug, Clone, Serialize)]
pub struct SimilarOrganization {
    pub organization_id: OrganizationId,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub organization_type_code: String,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    /// Adhésions **actives** seulement : une demande en attente ne prouve rien.
    pub member_count: i32,
    /// Dénomination qui a déclenché la correspondance — « trouvée par son
    /// sigle ». Nulle quand la fiche est entrée par le seul domaine.
    pub matched_name: Option<String>,
    /// Le front le compare à un seuil qu'il porte lui-même (85). Rendu tel que
    /// la base le calcule.
    pub score: f64,
    pub match_reasons: Vec<String>,
}

impl SimilarOrganization {
    /// La fiche est-elle entrée par une ressemblance de **dénomination** ?
    ///
    /// C'est le filtre de la lecture destinée à une personne. Il porte sur le
    /// motif et non sur le score : le motif n'est posé qu'au-dessus de 0,3 quand
    /// l'opérateur trigramme fait entrer à partir de 0,3, et filtrer sur le
    /// score garderait une ligne que l'écran écarte.
    pub fn matched_by_name(&self) -> bool {
        self.match_reasons
            .iter()
            .any(|r| r == MATCH_NAME_SIMILARITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_limite_est_bornee_des_deux_cotes() {
        assert_eq!(bounded_limit(None), DEFAULT_LIMIT);
        assert_eq!(bounded_limit(Some(0)), DEFAULT_LIMIT);
        assert_eq!(bounded_limit(Some(-3)), DEFAULT_LIMIT);
        assert_eq!(bounded_limit(Some(7)), 7);
        assert_eq!(bounded_limit(Some(100_000)), MAX_LIMIT);
    }
}
