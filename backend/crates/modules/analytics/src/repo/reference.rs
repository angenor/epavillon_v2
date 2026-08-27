//! `reference.taxonomy_terms` et `reference.countries` — **hors `cross/`**.
//!
//! Même raison que `settings.rs` : le principe III nomme `reference` comme
//! noyau partagé. Les libellés et les couleurs des répartitions viennent de là
//! et **jamais d'un fichier i18n** — figer les couleurs de thématique dans la
//! feuille de style est le défaut n° 1 de la v1.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use std::collections::HashMap;

/// Libellé et couleur d'un terme, par code.
pub struct LibelleDeTerme {
    pub label: Value,
    pub color_hex: Option<String>,
}

pub async fn thematiques(conn: &mut PgConnection) -> Result<HashMap<String, LibelleDeTerme>> {
    let lignes = sqlx::query!(
        r#"SELECT code, label AS "label!: Value", color_hex
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'activity_theme'"#
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.code,
                LibelleDeTerme {
                    label: l.label,
                    color_hex: l.color_hex,
                },
            )
        })
        .collect())
}

/// Libellé d'un pays, par code ISO 2. **Multilingue brut** : le site le résout.
pub async fn pays(conn: &mut PgConnection) -> Result<HashMap<String, Value>> {
    let lignes = sqlx::query!(
        r#"SELECT iso2 AS "iso2!", name AS "name!: Value"
             FROM reference.countries"#
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.iso2, l.name)).collect())
}
