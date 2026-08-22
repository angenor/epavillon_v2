//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use kernel::mail::Mailer;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct EngagementState {
    db: Db,
    config: Arc<Config>,
    /// L'expéditeur **déjà enveloppé** par la garde. Le module ne l'enveloppe
    /// pas lui-même : c'est l'API et le worker qui composent, une fois, pour
    /// que les courriels des modules livrés passent par la même porte.
    mailer: Arc<dyn Mailer>,
}

impl EngagementState {
    pub fn new(db: Db, config: Arc<Config>, mailer: Arc<dyn Mailer>) -> Self {
        Self { db, config, mailer }
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

    pub fn mailer(&self) -> &Arc<dyn Mailer> {
        &self.mailer
    }
}
