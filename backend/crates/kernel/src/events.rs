//! Émission d'événements de domaine, registre de consommateurs, garde
//! d'idempotence.
//!
//! Un événement s'écrit **dans la transaction du changement d'état**, par
//! `platform.emit_event()` et jamais par un INSERT direct : la fonction agrège
//! l'acteur dans `metadata` et l'identifiant de requête dans `correlation_id`,
//! et réveille le relais par `pg_notify`.

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::Result;

pub const OUTBOX_CHANNEL: &str = "platform_outbox";

/// La forme du type est imposée par `ck_outbox_event_type_format` : trois
/// segments exactement. Le code ne la revalide pas, il traduit l'échec.
pub struct DomainEvent<'a> {
    pub aggregate_schema: &'a str,
    pub aggregate_type: &'a str,
    pub aggregate_id: Uuid,
    pub event_type: &'a str,
    pub payload: Value,
}

/// Aucun secret ne franchit l'outbox : la table est durable, indexée par
/// agrégat, faite pour être relue et rejouée.
pub async fn emit(conn: &mut PgConnection, event: DomainEvent<'_>) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        "SELECT platform.emit_event($1, $2, $3, $4, $5, '{}'::jsonb)",
        event.aggregate_schema,
        event.aggregate_type,
        event.aggregate_id,
        event.event_type,
        event.payload
    )
    .fetch_one(conn)
    .await?;

    Ok(id.expect("platform.emit_event() rend toujours un identifiant"))
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub aggregate_schema: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Value,
    pub metadata: Value,
    pub correlation_id: Option<String>,
    pub occurred_at: OffsetDateTime,
}

#[async_trait]
pub trait EventConsumer: Send + Sync {
    /// Nom inscrit dans `platform.inbox_events` : il identifie le consommateur
    /// pour la garde d'idempotence, et ne se renomme pas à la légère.
    fn name(&self) -> &'static str;

    fn handles(&self, event_type: &str) -> bool {
        let _ = event_type;
        true
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()>;
}

#[derive(Default, Clone)]
pub struct ConsumerRegistry {
    consumers: Vec<std::sync::Arc<dyn EventConsumer>>,
}

impl ConsumerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(mut self, consumer: impl EventConsumer + 'static) -> Self {
        self.consumers.push(std::sync::Arc::new(consumer));
        self
    }

    pub fn interested<'a>(
        &'a self,
        event_type: &'a str,
    ) -> impl Iterator<Item = &'a dyn EventConsumer> + 'a {
        self.consumers
            .iter()
            .filter(move |c| c.handles(event_type))
            .map(|c| c.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.consumers.is_empty()
    }
}

/// Réserve `(consommateur, événement)` dans `platform.inbox_events`.
/// Faux signifie « déjà traité » : on passe au suivant sans produire d'effet.
pub async fn claim(conn: &mut PgConnection, consumer: &str, event_id: Uuid) -> Result<bool> {
    let insere = sqlx::query!(
        "INSERT INTO platform.inbox_events (consumer, event_id)
         VALUES ($1, $2)
         ON CONFLICT (consumer, event_id) DO NOTHING",
        consumer,
        event_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(insere == 1)
}
