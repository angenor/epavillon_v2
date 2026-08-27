//! Les natures d'incident — `reference.taxonomy_terms`, taxonomie
//! `incident_kind`.
//!
//! **Hors `cross/`, et délibérément** : `reference` est le noyau partagé que le
//! principe III exempte nommément. Le ranger dans `cross/` ferait perdre au
//! dossier son sens, qui est de lister exactement les frontières à trancher le
//! jour où le module deviendrait un service autonome — or aucun découplage ne
//! couperait le référentiel.
//!
//! **Vocabulaire ouvert, jamais un ENUM** : l'IFDD complète la liste depuis le
//! back-office, et le code ne connaît aucune des neuf valeurs.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;

use crate::domain::desk::TaxonomyTerm;

/// Les termes **actifs**, dans leur `sort_order`.
///
/// Les inactifs sont écartés : ils ne sont pas supprimés — des messages anciens
/// les référencent — mais on ne les propose plus au choix.
pub async fn natures(conn: &mut PgConnection) -> Result<Vec<TaxonomyTerm>> {
    let lignes = sqlx::query!(
        r#"SELECT id, taxonomy_code, parent_id, code,
                  label AS "label!: Value", description AS "description?: Value",
                  color_hex, icon, sort_order, is_active, superseded_by,
                  metadata AS "metadata!: Value",
                  created_at, updated_at
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'incident_kind' AND is_active
            ORDER BY sort_order, code"#
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| TaxonomyTerm {
            id: l.id,
            taxonomy_code: l.taxonomy_code,
            parent_id: l.parent_id,
            code: l.code,
            label: l.label,
            description: l.description,
            color_hex: l.color_hex,
            icon: l.icon,
            sort_order: l.sort_order,
            is_active: l.is_active,
            superseded_by: l.superseded_by,
            metadata: l.metadata,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}
