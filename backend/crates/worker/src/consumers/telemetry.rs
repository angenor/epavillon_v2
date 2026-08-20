//! Consommateur de télémétrie — une trace par événement.
//!
//! Le compteur annoncé par le contrat n'est PAS livré : monter un exportateur
//! de métriques OTLP, son intervalle et son arrêt propre pour un unique
//! compteur coûte plus que ce qu'il rapporte, alors que `count by event_type`
//! sur les traces donne déjà le chiffre. Écart consigné, et non chantier ouvert.
//!
//! Seul consommateur de ce jalon : `identity` est le seul module qui existe, il
//! n'y a personne à qui annoncer quoi que ce soit. Il n'est pas un pis-aller
//! pour autant : il rend visible dans Jaeger ce qui passe dans l'outbox, et
//! **il exerce la garde d'idempotence de bout en bout** — arrêter le worker, le
//! relancer sur mille événements déjà traités, et vérifier qu'aucun n'est
//! rejoué. Sans lui, FR-010 ne serait éprouvable qu'au module suivant.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

pub struct TelemetryConsumer;

#[async_trait]
impl EventConsumer for TelemetryConsumer {
    fn name(&self) -> &'static str {
        "telemetry"
    }

    async fn handle(&self, _conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        tracing::info!(
            evenement = %event.id,
            type_evenement = %event.event_type,
            agregat = %format!("{}.{}", event.aggregate_schema, event.aggregate_type),
            agregat_id = %event.aggregate_id,
            correlation_id = event.correlation_id.as_deref().unwrap_or("-"),
            "événement de domaine relayé"
        );
        Ok(())
    }
}
