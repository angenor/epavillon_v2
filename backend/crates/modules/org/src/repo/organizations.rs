//! Lectures et écritures de `org.organizations`.
//!
//! Ce que le dépôt **n'écrit jamais** : les colonnes engendrées
//! (`legal_name_normalized`, `acronym_normalized`), l'état de fusion
//! (`merged_into_id`, `merged_at` — posés par `org.merge_organizations()`), et
//! le score de confiance, qui appartient au travail différé de recalcul.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::ids::{OrganizationId, PersonId};
use crate::domain::organization::{Organization, OrganizationStatus};

/// Défaut et maximum de la liste ouverte. Seule la page de guide de style
/// l'appelle ; elle est livrée pour ne pas la casser, et bornée pour qu'elle ne
/// devienne pas un export du référentiel.
pub const LIST_DEFAULT_LIMIT: i64 = 50;
pub const LIST_MAX_LIMIT: i64 = 200;

/// Une fiche par son identifiant. Rendue **telle quelle**, absorbée comprise :
/// elle porte alors `merged_into_id`, et l'appelant sait quoi en faire.
pub async fn by_id<'e>(
    executor: impl PgExecutor<'e>,
    id: OrganizationId,
) -> Result<Option<Organization>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"SELECT id, legal_name, legal_name_normalized, acronym, acronym_normalized,
                  slug::text AS "slug!", organization_type_code, country_id, city,
                  description::jsonb AS "description?", website::text AS "website?",
                  contact_email::text AS "contact_email?", contact_phone,
                  status::text AS "statut!", merged_into_id, merged_at,
                  verified_at, verified_by, trust_score, created_by, created_at, updated_at
             FROM org.organizations WHERE id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    ligne.map(Organization::try_from).transpose()
}

/// La liste ouverte : fiches **vivantes** seulement, triées par nom légal.
pub async fn list(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Organization>> {
    let lignes = sqlx::query_as!(
        Ligne,
        r#"SELECT id, legal_name, legal_name_normalized, acronym, acronym_normalized,
                  slug::text AS "slug!", organization_type_code, country_id, city,
                  description::jsonb AS "description?", website::text AS "website?",
                  contact_email::text AS "contact_email?", contact_phone,
                  status::text AS "statut!", merged_into_id, merged_at,
                  verified_at, verified_by, trust_score, created_by, created_at, updated_at
             FROM org.organizations
            WHERE status IN ('candidate', 'active')
            ORDER BY legal_name
            LIMIT $1 OFFSET $2"#,
        limit.clamp(1, LIST_MAX_LIMIT),
        offset.max(0)
    )
    .fetch_all(pool)
    .await?;

    lignes.into_iter().map(Organization::try_from).collect()
}

/// La fiche vivante d'une organisation, en suivant la fusion.
///
/// `org.resolve_organization()` existe pour cela, et le trigger
/// `tg_organizations_no_merge_chain` garantit qu'il n'y a jamais de chaîne à
/// remonter. Suivre `merged_into_id` à la main serait réécrire la fonction.
pub async fn resolve<'e>(
    executor: impl PgExecutor<'e>,
    id: OrganizationId,
) -> Result<Option<OrganizationId>> {
    let resolu = sqlx::query_scalar!("SELECT org.resolve_organization($1)", id.as_uuid())
        .fetch_one(executor)
        .await?;

    Ok(resolu.map(OrganizationId))
}

pub struct NewOrganization<'a> {
    pub legal_name: &'a str,
    pub acronym: Option<&'a str>,
    pub slug: &'a str,
    pub organization_type_code: &'a str,
    pub country_id: Option<Uuid>,
    pub city: Option<&'a str>,
    pub website: Option<&'a str>,
    pub description: Option<&'a serde_json::Value>,
    pub created_by: PersonId,
}

/// Crée la fiche en `candidate`. **Jamais `active`** : une fiche née d'un
/// formulaire public n'est pas une fiche de référence tant que l'IFDD ne l'a pas
/// regardée.
///
/// Le nom légal et le sigle deviennent cherchables sans écriture du service :
/// `tg_organizations_sync_names` les recopie dans les dénominations.
pub async fn create(conn: &mut PgConnection, fiche: NewOrganization<'_>) -> Result<Organization> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, country_id, city,
                website, description, created_by, status)
           VALUES ($1, $2, $3::text::platform.slug, $4, $5, $6,
                   $7::text::platform.url, $8::jsonb::platform.i18n_text, $9, 'candidate')
        RETURNING id, legal_name, legal_name_normalized, acronym, acronym_normalized,
                  slug::text AS "slug!", organization_type_code, country_id, city,
                  description::jsonb AS "description?", website::text AS "website?",
                  contact_email::text AS "contact_email?", contact_phone,
                  status::text AS "statut!", merged_into_id, merged_at,
                  verified_at, verified_by, trust_score, created_by, created_at, updated_at"#,
        fiche.legal_name,
        fiche.acronym,
        fiche.slug,
        fiche.organization_type_code,
        fiche.country_id,
        fiche.city,
        fiche.website,
        fiche.description,
        fiche.created_by.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Organization::try_from(ligne)
}

/// Pose le sceau, et **admet du même geste** une fiche `candidate`.
///
/// Rend l'instant posé, ou `None` si la fiche n'existe pas. Le sceau déjà posé
/// n'est pas déplacé : reposer la date ferait mentir l'historique.
pub async fn set_verified(
    conn: &mut PgConnection,
    id: OrganizationId,
    verifie_par: PersonId,
) -> Result<Option<time::OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        "UPDATE org.organizations
            SET verified_at = COALESCE(verified_at, now()),
                verified_by = COALESCE(verified_by, $2),
                status = CASE WHEN status = 'candidate' THEN 'active' ELSE status END
          WHERE id = $1
      RETURNING verified_at",
        id.as_uuid(),
        verifie_par.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(instant.flatten())
}

/// Retire le sceau. **Ne change pas le statut** : la fiche reste active, elle
/// cesse d'être certifiée. Les confondre ferait disparaître d'un écran une
/// organisation qu'on voulait seulement ne pas mettre en avant.
pub async fn clear_verified(conn: &mut PgConnection, id: OrganizationId) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE org.organizations SET verified_at = NULL, verified_by = NULL WHERE id = $1",
        id.as_uuid()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Le statut d'une fiche, sans la lire en entier.
pub async fn status_of<'e>(
    executor: impl PgExecutor<'e>,
    id: OrganizationId,
) -> Result<Option<OrganizationStatus>> {
    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "statut!" FROM org.organizations WHERE id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    statut.map(|s| convertir_statut(&s)).transpose()
}

pub(crate) fn convertir_statut(valeur: &str) -> Result<OrganizationStatus> {
    OrganizationStatus::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("statut d'organisation inconnu : {valeur}")))
}

/// Les vingt-deux colonnes de la table, transportées telles quelles.
pub(crate) struct Ligne {
    pub id: Uuid,
    pub legal_name: String,
    pub legal_name_normalized: Option<String>,
    pub acronym: Option<String>,
    pub acronym_normalized: Option<String>,
    pub slug: String,
    pub organization_type_code: String,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub description: Option<serde_json::Value>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub statut: String,
    pub merged_into_id: Option<Uuid>,
    pub merged_at: Option<time::OffsetDateTime>,
    pub verified_at: Option<time::OffsetDateTime>,
    pub verified_by: Option<Uuid>,
    pub trust_score: i16,
    pub created_by: Option<Uuid>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl TryFrom<Ligne> for Organization {
    type Error = ApiError;

    fn try_from(l: Ligne) -> Result<Self> {
        Ok(Self {
            id: OrganizationId(l.id),
            legal_name: l.legal_name,
            legal_name_normalized: l.legal_name_normalized,
            acronym: l.acronym,
            acronym_normalized: l.acronym_normalized,
            slug: l.slug,
            organization_type_code: l.organization_type_code,
            country_id: l.country_id,
            city: l.city,
            description: l.description,
            website: l.website,
            contact_email: l.contact_email,
            contact_phone: l.contact_phone,
            status: convertir_statut(&l.statut)?,
            merged_into_id: l.merged_into_id.map(OrganizationId),
            merged_at: l.merged_at,
            verified_at: l.verified_at,
            verified_by: l.verified_by.map(PersonId),
            trust_score: l.trust_score,
            created_by: l.created_by.map(PersonId),
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
    }
}
