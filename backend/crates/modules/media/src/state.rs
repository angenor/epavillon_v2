//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use sqlx::PgPool;
use std::sync::Arc;

use crate::scan::Scanner;
use crate::storage::ObjectStore;

#[derive(Clone)]
pub struct MediaState {
    db: Db,
    config: Arc<Config>,
    /// Le stockage choisi par la configuration. Le service ne sait pas par où
    /// passent les octets — même patron que `kernel::mail`.
    storage: Arc<dyn ObjectStore>,
    scanner: Arc<dyn Scanner>,
}

impl MediaState {
    pub fn new(db: Db, config: Arc<Config>) -> Self {
        let storage = crate::storage::build(&config.media);
        let scanner = crate::scan::build(&config.media);
        Self {
            db,
            config,
            storage,
            scanner,
        }
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

    pub fn storage(&self) -> &Arc<dyn ObjectStore> {
        &self.storage
    }

    pub fn scanner(&self) -> &Arc<dyn Scanner> {
        &self.scanner
    }
}
