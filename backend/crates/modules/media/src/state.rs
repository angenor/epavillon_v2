//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use sqlx::PgPool;
use std::sync::Arc;

use crate::scan::Scanner;
use crate::storage::{Entrepots, ObjectStore};

#[derive(Clone)]
pub struct MediaState {
    db: Db,
    config: Arc<Config>,
    /// Le stockage choisi par la configuration, un par bucket. Le service ne
    /// sait pas par où passent les octets — même patron que `kernel::mail`.
    entrepots: Entrepots,
    scanner: Arc<dyn Scanner>,
}

impl MediaState {
    pub fn new(db: Db, config: Arc<Config>) -> Self {
        let entrepots = Entrepots::new(&config.media);
        let scanner = crate::scan::build(&config.media);
        Self {
            db,
            config,
            entrepots,
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

    /// Le stockage du bucket public.
    pub fn storage(&self) -> &Arc<dyn ObjectStore> {
        self.entrepots.defaut()
    }

    pub fn stockage(&self, bucket: &str) -> Arc<dyn ObjectStore> {
        self.entrepots.du_bucket(bucket)
    }

    pub fn entrepots(&self) -> &Entrepots {
        &self.entrepots
    }

    pub fn scanner(&self) -> &Arc<dyn Scanner> {
        &self.scanner
    }
}
