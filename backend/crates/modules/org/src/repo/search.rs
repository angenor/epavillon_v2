//! Les **deux lectures** de la recherche, déclarées côte à côte.
//!
//! C'est l'écart n° 23, et c'est le cœur du module. La fonction du modèle,
//! `org.find_similar_organizations()`, n'est pas modifiée : les deux lectures
//! l'enveloppent différemment.
//!
//! | | [`filtree`] | [`brute`] |
//! |---|---|---|
//! | **La question posée** | « Ce que j'ai tapé, est-ce que ça existe déjà ? » | « Qu'est-ce qui pourrait être la même entité ? » |
//! | **Qui la pose** | Une personne, sur l'écran de rattachement ou de dépôt | Le back-office, et le balayage de détection |
//! | **Filtre** | Seules les fiches portant `name_similarity` | **Aucun** |
//! | **Le domaine de l'appelant** | Alimente le score, **ne fait pas entrer** une fiche sans rapport | Fait entrer la fiche : c'est le signal le plus fiable |
//!
//! Chercher « Agence spatiale du Sahel » ne doit pas ramener l'organisation du
//! domaine de la personne, qu'un bandeau lui propose déjà nommément. Mais deux
//! fiches qui déclarent `osed-sahel.org` sont la même maison, quels que soient
//! les libellés saisis — et c'est ce que le back-office vient chercher.
//!
//! **Le filtre est en SQL et non en Rust** : filtrer côté application coûterait
//! le même aller-retour et ferait vivre la règle à deux endroits — cette
//! lecture-ci et le balayage de détection, qui appelle la même fonction avec
//! l'intention inverse. En SQL, la différence tient dans une ligne, lisible à
//! côté de l'autre (research.md § R1).
//!
//! **Le filtre porte sur le motif, pas sur le score** : le motif n'est posé
//! qu'au-dessus de 0,3 quand l'opérateur trigramme fait entrer à partir de 0,3.
//! Filtrer sur le score garderait une ligne que l'écran écarte, et l'API
//! divergerait de l'interface sur un cas invisible (écart n° 77).

use kernel::error::Result;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::OrganizationId;
use crate::domain::search::SimilarOrganization;

/// Ce que la fonction du modèle prend. Les quatre signaux, plus la limite.
#[derive(Debug, Clone)]
pub struct SearchInput<'a> {
    pub name: &'a str,
    pub country_id: Option<Uuid>,
    pub email: Option<&'a str>,
    pub website: Option<&'a str>,
    pub limit: i32,
}

/// Lecture destinée à **une personne** : seules les fiches dont le nom
/// ressemble à ce qui a été tapé.
///
/// L'appelant sur-lit (limite + marge) puis tronque — la limite étant appliquée
/// à l'intérieur de la fonction, filtrer après coup rendrait moins de lignes que
/// demandé. Voir [`crate::service::search`].
pub async fn filtree<'e>(
    executor: impl PgExecutor<'e>,
    input: SearchInput<'_>,
) -> Result<Vec<SimilarOrganization>> {
    let lignes = sqlx::query_as!(
        Ligne,
        r#"SELECT organization_id        AS "organization_id!",
                  legal_name             AS "legal_name!",
                  acronym,
                  organization_type_code AS "organization_type_code!",
                  country_id,
                  city,
                  status::text           AS "status!",
                  verified_at,
                  member_count           AS "member_count!",
                  matched_name,
                  score::float8          AS "score!",
                  match_reasons          AS "match_reasons!"
             FROM org.find_similar_organizations($1, $2, $3, $4, $5)
            WHERE 'name_similarity' = ANY(match_reasons)"#,
        input.name,
        input.country_id,
        input.email,
        input.website,
        input.limit
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(SimilarOrganization::from).collect())
}

/// Lecture destinée à **la revue des doublons** : rien n'est écarté.
///
/// Le domaine partagé fait entrer la fiche, et c'est voulu — c'est le signal le
/// plus fiable du modèle. C'est aussi la lecture qu'emploie le balayage de fond.
pub async fn brute<'e>(
    executor: impl PgExecutor<'e>,
    input: SearchInput<'_>,
) -> Result<Vec<SimilarOrganization>> {
    let lignes = sqlx::query_as!(
        Ligne,
        r#"SELECT organization_id        AS "organization_id!",
                  legal_name             AS "legal_name!",
                  acronym,
                  organization_type_code AS "organization_type_code!",
                  country_id,
                  city,
                  status::text           AS "status!",
                  verified_at,
                  member_count           AS "member_count!",
                  matched_name,
                  score::float8          AS "score!",
                  match_reasons          AS "match_reasons!"
             FROM org.find_similar_organizations($1, $2, $3, $4, $5)"#,
        input.name,
        input.country_id,
        input.email,
        input.website,
        input.limit
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(SimilarOrganization::from).collect())
}

/// Les treize colonnes de la fonction, transportées telles quelles. Le score
/// est transtypé en base : le contrat du front le compare à un seuil qu'il
/// porte lui-même, il n'a pas à être réinterprété en chemin.
struct Ligne {
    organization_id: Uuid,
    legal_name: String,
    acronym: Option<String>,
    organization_type_code: String,
    country_id: Option<Uuid>,
    city: Option<String>,
    status: String,
    verified_at: Option<OffsetDateTime>,
    member_count: i32,
    matched_name: Option<String>,
    score: f64,
    match_reasons: Vec<String>,
}

impl From<Ligne> for SimilarOrganization {
    fn from(l: Ligne) -> Self {
        Self {
            organization_id: OrganizationId(l.organization_id),
            legal_name: l.legal_name,
            acronym: l.acronym,
            organization_type_code: l.organization_type_code,
            country_id: l.country_id,
            city: l.city,
            status: l.status,
            verified_at: l.verified_at,
            member_count: l.member_count,
            matched_name: l.matched_name,
            score: l.score,
            match_reasons: l.match_reasons,
        }
    }
}
