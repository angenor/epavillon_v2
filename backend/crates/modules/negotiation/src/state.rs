//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use kernel::storage::Entrepots;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct NegotiationState {
    db: Db,
    config: Arc<Config>,
    /// Le PDF et les images de page d'un document vivent dans le bucket privé :
    /// l'API les sert elle-même, après avoir vérifié l'accès.
    entrepots: Entrepots,
}

impl NegotiationState {
    pub fn new(db: Db, config: Arc<Config>) -> Self {
        let entrepots = Entrepots::new(&config.media);
        Self {
            db,
            config,
            entrepots,
        }
    }

    pub fn entrepots(&self) -> &Entrepots {
        &self.entrepots
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn pool(&self) -> &PgPool {
        self.db.pool()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
