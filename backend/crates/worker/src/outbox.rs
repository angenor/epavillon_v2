//! Relais d'outbox : notification **et** balayage.
//!
//! `platform.emit_event()` appelle déjà `pg_notify` — c'est ce qui évite
//! d'attendre le prochain cycle. Mais une notification est perdue si le relais
//! est arrêté au moment où elle part, et elle n'est pas rejouée. Le balayage
//! est le filet ; la notification est la vitesse.

use kernel::db::Db;
use kernel::error::Result;
use kernel::events::{claim, ConsumerRegistry, OutboxEvent, OUTBOX_CHANNEL};
use sqlx::postgres::PgListener;
use std::time::Duration;
use uuid::Uuid;

const BALAYAGE: Duration = Duration::from_secs(10);
const LOT: i64 = 50;

pub async fn run(db: Db, registry: ConsumerRegistry) -> Result<()> {
    let mut ecoute = PgListener::connect_with(db.pool()).await?;
    ecoute.listen(OUTBOX_CHANNEL).await?;

    loop {
        if let Err(e) = relayer_lot(&db, &registry).await {
            tracing::error!(erreur = %e, "relais d'outbox interrompu");
        }

        // Le premier des deux qui parle réveille le tour suivant.
        tokio::select! {
            recu = ecoute.recv() => {
                if let Err(e) = recu {
                    tracing::warn!(erreur = %e, "écoute de l'outbox rompue, reprise par balayage");
                    tokio::time::sleep(BALAYAGE).await;
                }
            }
            _ = tokio::time::sleep(BALAYAGE) => {}
        }
    }
}

async fn relayer_lot(db: &Db, registry: &ConsumerRegistry) -> Result<()> {
    let candidats = sqlx::query_scalar!(
        "SELECT id FROM platform.outbox_events
          WHERE published_at IS NULL AND available_at <= now()
          ORDER BY available_at, id
          LIMIT $1",
        LOT
    )
    .fetch_all(db.pool())
    .await?;

    for id in candidats {
        if let Err(e) = relayer_un(db, registry, id).await {
            tracing::error!(evenement = %id, erreur = %e, "événement non relayé");
            repousser(db, id, &e.to_string()).await?;
        }
    }
    Ok(())
}

/// Une transaction par événement : l'échec de l'un ne rejoue pas les autres.
/// La réservation `FOR UPDATE SKIP LOCKED` fait que plusieurs relais ne se
/// marchent pas dessus.
async fn relayer_un(db: &Db, registry: &ConsumerRegistry, id: Uuid) -> Result<()> {
    let span = tracing::info_span!("outbox", evenement = %id);
    let _garde = span.enter();

    let ctx = kernel::RequestContext::background("outbox");
    let mut tx = db.write(&ctx).await?;

    let Some(ligne) = sqlx::query!(
        "SELECT id, aggregate_schema, aggregate_type, aggregate_id, event_type,
                event_version, payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events
          WHERE id = $1 AND published_at IS NULL
            FOR UPDATE SKIP LOCKED",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(());
    };

    let evenement = OutboxEvent {
        id: ligne.id,
        aggregate_schema: ligne.aggregate_schema,
        aggregate_type: ligne.aggregate_type,
        aggregate_id: ligne.aggregate_id,
        event_type: ligne.event_type,
        event_version: ligne.event_version,
        payload: ligne.payload,
        metadata: ligne.metadata,
        correlation_id: ligne.correlation_id,
        occurred_at: ligne.occurred_at,
    };

    for consommateur in registry.interested(&evenement.event_type) {
        // Garde d'idempotence : un événement déjà traité par ce consommateur
        // ne produit aucun effet une seconde fois.
        if claim(&mut tx, consommateur.name(), evenement.id).await? {
            consommateur.handle(&mut tx, &evenement).await?;
        }
    }

    sqlx::query!(
        "UPDATE platform.outbox_events SET published_at = now() WHERE id = $1",
        evenement.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

/// Délai croissant plafonné à cinq minutes. Ce n'est pas la reprise d'essai de
/// `platform.fail_job()` réécrite : celle-là porte les TRAVAUX. L'outbox n'a ni
/// fonction de replanification ni file morte dans le modèle — un événement non
/// relayé doit finir par l'être.
async fn repousser(db: &Db, id: Uuid, erreur: &str) -> Result<()> {
    let mut tx = db
        .write(&kernel::RequestContext::background("outbox"))
        .await?;
    sqlx::query!(
        "UPDATE platform.outbox_events
            SET attempts = attempts + 1,
                last_error = $2,
                available_at = now() + (least(power(2, attempts), 300) || ' seconds')::interval
          WHERE id = $1",
        id,
        erreur
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}
