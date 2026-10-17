//! Ce dont les routes du module ont besoin, et rien de plus.

use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::PgPool;
use std::sync::Arc;

use crate::zoom::Zoom;

#[derive(Clone)]
pub struct LiveState {
    db: Db,
    config: Arc<Config>,
    zoom: Option<Arc<Zoom>>,
}

impl LiveState {
    pub fn new(db: Db, config: Arc<Config>) -> Self {
        let zoom = zoom_depuis(&config);
        Self { db, config, zoom }
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

    /// Zoom, ou le refus que toute route qui en a besoin rend.
    pub fn zoom(&self) -> Result<&Zoom> {
        self.zoom
            .as_deref()
            .ok_or_else(|| ApiError::new(ErrorCode::LiveZoomUnavailable))
    }

    /// Pour les tests : un Zoom construit à la main (adresse de l'API locale).
    pub fn avec_zoom(mut self, zoom: Zoom) -> Self {
        self.zoom = Some(Arc::new(zoom));
        self
    }
}

pub fn zoom_depuis(config: &Config) -> Option<Arc<Zoom>> {
    let reglages = config.zoom.as_ref()?;
    match Zoom::depuis(reglages) {
        Ok(z) => Some(Arc::new(z)),
        Err(e) => {
            tracing::error!(erreur = %e, "Zoom configuré mais inutilisable : visioconférence éteinte");
            None
        }
    }
}
