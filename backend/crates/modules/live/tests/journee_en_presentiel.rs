//! Une journée passée en présentiel seulement (07/11) : l'annonce
//! `event.track.streaming_disabled` annule sa réunion et pose son annulation
//! chez Zoom ; aucune réunion ne renaît pour elle.

mod commun;

use commun::{decor, Bac};
use kernel::context::RequestContext;
use kernel::events::{EventConsumer, OutboxEvent};
use live::consumers::journees::JourneesConsumer;
use live::repo::cross::programme;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn eteindre_le_direct_annule_la_reunion_de_la_journee() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let fil: Uuid = sqlx::query_scalar(
        "INSERT INTO event.programme_tracks (event_id, code, slug, kind, title, published_at)
         VALUES ($1, 'atelier', 'atelier', 'special_day', '{\"fr\":\"Atelier\"}', now())
         RETURNING id",
    )
    .bind(d.event_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    let reunion: Uuid = sqlx::query_scalar(
        "INSERT INTO live.meetings
             (track_id, event_id, provider, external_id, topic, scheduled_start_at,
              duration_minutes, timezone, requires_registration, status, sync_status)
         VALUES ($1, $2, 'zoom', '85746065999', 'Atelier', now() + interval '1 day', 360,
                 $3, true, 'scheduled', 'synced')
         RETURNING id",
    )
    .bind(fil)
    .bind(d.event_id)
    .bind(&d.timezone)
    .fetch_one(bac.pool())
    .await
    .unwrap();

    sqlx::query("UPDATE event.programme_tracks SET is_streamed = false WHERE id = $1")
        .bind(fil)
        .execute(bac.pool())
        .await
        .unwrap();
    let evenement = OutboxEvent {
        id: Uuid::now_v7(),
        aggregate_schema: "event".into(),
        aggregate_type: "track".into(),
        aggregate_id: fil,
        event_type: contracts::event::TRACK_STREAMING_DISABLED.into(),
        event_version: 1,
        payload: json!({}),
        metadata: json!({}),
        correlation_id: None,
        occurred_at: OffsetDateTime::now_utc(),
    };
    assert!(JourneesConsumer.handles(&evenement.event_type));

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    JourneesConsumer.handle(&mut tx, &evenement).await.unwrap();
    assert!(
        programme::journee(&mut tx, fil).await.unwrap().is_none(),
        "une journée éteinte n'existe pas pour le direct"
    );
    tx.commit().await.unwrap();

    let statut: String = sqlx::query_scalar("SELECT status::text FROM live.meetings WHERE id = $1")
        .bind(reunion)
        .fetch_one(bac.pool())
        .await
        .unwrap();
    assert_eq!(statut, "cancelled");
    let travaux: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.jobs
          WHERE task = 'live.cancel_meeting' AND payload ->> 'meeting_id' = $1::text",
    )
    .bind(reunion)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(travaux, 1, "annulation posée pour Zoom");
}
