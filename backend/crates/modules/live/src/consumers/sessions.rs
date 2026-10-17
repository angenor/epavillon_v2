//! `programme.session.cancelled` et `programme.session.rescheduled` : la
//! réunion Zoom suit sa séance (annulée chez Zoom, ou déplacée). Une séance
//! reportée (`postponed`) garde sa réunion : l'accès est refusé, rien n'est
//! défait chez Zoom.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::repo::cross::programme;
use crate::repo::meetings;
use crate::service::inscrits;

pub const ANNULEE: &str = "programme.session.cancelled";
pub const DEPLACEE: &str = "programme.session.rescheduled";

pub struct SessionsConsumer;

#[async_trait]
impl EventConsumer for SessionsConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "live.sessions"
    }

    fn handles(&self, event_type: &str) -> bool {
        matches!(event_type, ANNULEE | DEPLACEE)
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let Some(seance) = programme::seance(conn, event.aggregate_id).await? else {
            return Ok(());
        };
        if seance.status == "cancelled" {
            return meetings::annuler_reunion(conn, seance.id).await;
        }
        if event.event_type == DEPLACEE {
            let minutes = inscrits::duree_minutes(&seance);
            return meetings::recaler_reunion(conn, seance.id, seance.starts_at, minutes).await;
        }
        Ok(())
    }
}
