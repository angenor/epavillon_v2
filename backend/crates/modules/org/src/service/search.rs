//! La recherche, ses deux lectures et la sur-lecture.

use kernel::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::search::{bounded_limit, SimilarOrganization, MIN_TERM_LEN, OVERREAD_MARGIN};
use crate::repo::search::{self, SearchInput};

/// Ce qu'une route reçoit, avant bornage.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub name: String,
    pub country_id: Option<Uuid>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub limit: Option<i32>,
}

/// Recherche destinée à **une personne** : sur-lecture, filtre, troncature.
///
/// La limite est appliquée à l'intérieur de la fonction du modèle. Demander dix
/// et en écarter trois rendrait sept résultats, alors que trois autres fiches
/// attendaient derrière : on demande donc `limite + marge`, puis on tronque.
///
/// **Un terme trop court rend une liste vide, jamais une erreur** : le front ne
/// le demande pas, et le garde existe pour qu'un appel forgé ne balaie pas la
/// table (FR-013).
pub async fn similar_for_person(
    pool: &PgPool,
    query: SearchQuery,
) -> Result<Vec<SimilarOrganization>> {
    let terme = query.name.trim();
    let limite = bounded_limit(query.limit);

    if terme.chars().count() < MIN_TERM_LEN {
        return Ok(Vec::new());
    }

    let mut resultats = search::filtree(
        pool,
        SearchInput {
            name: terme,
            country_id: query.country_id,
            email: query.email.as_deref(),
            website: query.website.as_deref(),
            limit: limite + OVERREAD_MARGIN,
        },
    )
    .await?;

    resultats.truncate(limite as usize);
    Ok(resultats)
}

/// Recherche destinée à **la revue des doublons** : aucun filtre, aucune
/// sur-lecture — la limite demandée est celle que la fonction applique.
pub async fn similar_for_review(
    pool: &PgPool,
    query: SearchQuery,
) -> Result<Vec<SimilarOrganization>> {
    let terme = query.name.trim();
    let limite = bounded_limit(query.limit);

    // Le même garde, et pour la même raison : le back-office n'a pas plus de
    // titre qu'un visiteur à faire balayer la table par un terme d'un signe.
    if terme.chars().count() < MIN_TERM_LEN {
        return Ok(Vec::new());
    }

    search::brute(
        pool,
        SearchInput {
            name: terme,
            country_id: query.country_id,
            email: query.email.as_deref(),
            website: query.website.as_deref(),
            limit: limite,
        },
    )
    .await
}
