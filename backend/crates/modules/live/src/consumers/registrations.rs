//! `programme.registration.*` : chaque inscription qui a une place devient un
//! inscrit Zoom, chaque annulation libère la sienne.
//!
//! Le type d'événement porte le STATUT (`programme.registration.cancelled`),
//! jamais `confirmed` (écart n° 126) : on écoute tout le préfixe et on relit
//! l'inscription en base.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::service::inscrits;

pub const PREFIXE: &str = "programme.registration.";

pub struct RegistrationsConsumer;

#[async_trait]
impl EventConsumer for RegistrationsConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "live.registrations"
    }

    fn handles(&self, event_type: &str) -> bool {
        event_type.starts_with(PREFIXE)
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        inscrits::synchroniser(conn, event.aggregate_id).await
    }
}
