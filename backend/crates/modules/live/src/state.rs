//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct LiveState {
    db: Db,
    config: Arc<Config>,
}

impl LiveState {
    pub fn new(db: Db, config: Arc<Config>) -> Self {
        Self { db, config }
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
