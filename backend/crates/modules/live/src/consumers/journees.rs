//! `programme.track_registration.*` (02/11) : chaque inscription à une journée
//! prend une place dans la réunion de la journée, née avec la première ;
//! chaque annulation libère la sienne. Le statut se relit en base.
//!
//! `event.track.streaming_disabled` (07/11) : la journée passe en présentiel
//! seulement, sa réunion est annulée.

use async_trait::async_trait;
use contracts::event::TRACK_STREAMING_DISABLED;
use contracts::programme::TRACK_REGISTRATION_PREFIX;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::repo::meetings;
use crate::service::inscrits;

pub struct JourneesConsumer;

#[async_trait]
impl EventConsumer for JourneesConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "live.journees"
    }

    fn handles(&self, event_type: &str) -> bool {
        event_type.starts_with(TRACK_REGISTRATION_PREFIX) || event_type == TRACK_STREAMING_DISABLED
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        if event.event_type == TRACK_STREAMING_DISABLED {
            return meetings::annuler_reunion_journee(conn, event.aggregate_id).await;
        }
        inscrits::synchroniser_journee(conn, event.aggregate_id).await
    }
}
