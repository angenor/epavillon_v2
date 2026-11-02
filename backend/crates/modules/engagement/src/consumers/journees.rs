//! Les rappels d'une journée (02/11) : posés à l'inscription, reposés quand
//! une de ses activités est programmée ou déplacée (le début de la journée peut
//! en dépendre). Les travaux d'une heure dépassée ne partent pas d'eux-mêmes.
//! Une désinscription n'a rien à annuler : le travail relit l'inscription.

use async_trait::async_trait;
use contracts::programme::TRACK_REGISTRATION_PREFIX;
use kernel::db::Db;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::jobs::journees::poser_rappels;
use crate::repo::journees;

pub struct JourneesConsumer {
    db: Db,
}

impl JourneesConsumer {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EventConsumer for JourneesConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "engagement.journees"
    }

    fn handles(&self, event_type: &str) -> bool {
        event_type.starts_with(TRACK_REGISTRATION_PREFIX)
            || matches!(
                event_type,
                contracts::programme::SESSION_CREATED
                    | contracts::programme::SESSION_SCHEDULED
                    | contracts::programme::SESSION_RESCHEDULED
            )
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let pool = self.db.pool();
        if event.event_type.starts_with(TRACK_REGISTRATION_PREFIX) {
            poser_rappels(pool, conn, event.aggregate_id).await?;
            return Ok(());
        }
        for id in journees::inscriptions_de_la_journee_de(pool, event.aggregate_id).await? {
            poser_rappels(pool, conn, id).await?;
        }
        Ok(())
    }
}
