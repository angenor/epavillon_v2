//! Les annonces de la régie du jour (02/11) deviennent des travaux à l'heure
//! dite : l'annulation part cinq minutes après le geste, le « finalement
//! maintenue » tout de suite. Le travail vérifie lui-même, au départ, que
//! l'annulation tient encore.

use async_trait::async_trait;
use contracts::programme::{RegieNotice, REGIE_CANCELLATION_NOTICE, REGIE_RESTORATION_NOTICE};
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::jobs::regie_notice::{self, ANNULEE, MAINTENUE};

pub struct RegieConsumer;

#[async_trait]
impl EventConsumer for RegieConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "engagement.regie"
    }

    fn handles(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            REGIE_CANCELLATION_NOTICE | REGIE_RESTORATION_NOTICE
        )
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let avis: RegieNotice = match serde_json::from_value(event.payload.clone()) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(event_type = %event.event_type, erreur = %e, "annonce de régie illisible");
                return Ok(());
            }
        };
        let kind = if event.event_type == REGIE_CANCELLATION_NOTICE {
            ANNULEE
        } else {
            MAINTENUE
        };
        regie_notice::poser(conn, kind, avis.action_id, avis.session_id, avis.notify_at).await
    }
}
