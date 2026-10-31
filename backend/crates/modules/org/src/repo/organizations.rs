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

/// Les champs que l'équipe corrige (31/10). Rend les colonnes changées, `None`
/// si la fiche n'existe pas. `tg_organizations_sync_names` fait de l'ancien nom
/// et de l'ancien sigle des anciens noms, toujours cherchables.
pub async fn update_profile(
    conn: &mut PgConnection,
    id: OrganizationId,
    legal_name: &str,
    acronym: Option<&str>,
    organization_type_code: &str,
    country_id: Option<Uuid>,
    website: Option<&str>,
) -> Result<Option<Vec<String>>> {
    let ligne = sqlx::query!(
        r#"WITH avant AS (
               SELECT legal_name, acronym, organization_type_code, country_id, website::text AS website
                 FROM org.organizations WHERE id = $1 FOR UPDATE
           ), apres AS (
               UPDATE org.organizations
                  SET legal_name = $2, acronym = $3, organization_type_code = $4,
                      country_id = $5, website = $6::text::platform.url
                WHERE id = $1
            RETURNING id
           )
           SELECT array_remove(ARRAY[
                      CASE WHEN a.legal_name IS DISTINCT FROM $2 THEN 'legal_name' END,
                      CASE WHEN a.acronym IS DISTINCT FROM $3 THEN 'acronym' END,
                      CASE WHEN a.organization_type_code IS DISTINCT FROM $4 THEN 'organization_type_code' END,
                      CASE WHEN a.country_id IS DISTINCT FROM $5 THEN 'country_id' END,
                      CASE WHEN a.website IS DISTINCT FROM $6 THEN 'website' END
                  ], NULL) AS "changes!"
             FROM avant a, apres"#,
        id.as_uuid(),
        legal_name,
        acronym,
        organization_type_code,
        country_id,
        website
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| l.changes))
}

/// Le type d'organisation existe-t-il, et sert-il encore ?
pub async fn type_is_active(conn: &mut PgConnection, code: &str) -> Result<bool> {
    let actif = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM reference.taxonomy_terms
                WHERE taxonomy_code = 'organization_type' AND code = $1 AND is_active
           ) AS "actif!""#,
        code
    )
    .fetch_one(conn)
    .await?;
    Ok(actif)
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
