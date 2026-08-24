//! Montage conditionnel des routes, d'après `platform.modules`.
//!
//! Un module non monté ne répond pas 403 mais **404** : ses chemins n'existent
//! pas. Un 403 dirait qu'il existe et qu'on n'y a pas droit — deux choses
//! différentes, et la seconde est fausse.

use kernel::error::Result;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    montes: Arc<HashSet<String>>,
}

impl ModuleRegistry {
    pub async fn load(pool: &PgPool) -> Result<Self> {
        let codes =
            sqlx::query_scalar!("SELECT code FROM platform.modules WHERE deployment <> 'disabled'")
                .fetch_all(pool)
                .await?;

        Ok(Self {
            montes: Arc::new(codes.into_iter().collect()),
        })
    }

    pub fn is_mounted(&self, code: &str) -> bool {
        self.montes.contains(code)
    }
}

impl ModuleRegistry {
    /// Tous les modules montés, sans consulter la base.
    ///
    /// Réservé à l'**export** du document OpenAPI (`cargo run -p api --bin
    /// openapi`) : la documentation décrit la surface complète de l'API, alors
    /// qu'une application réelle ne monte que ce que `platform.modules` déclare.
    /// Engendrer le document depuis une base de développement le ferait varier
    /// avec l'état de cette base — un module éteint le jour de la génération
    /// retirerait ses chemins du client TypeScript du site.
    pub fn complet() -> Self {
        Self {
            montes: Arc::new(
                [
                    "identity",
                    "org",
                    "event",
                    "programme",
                    "media",
                    "engagement",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ),
        }
    }
}
