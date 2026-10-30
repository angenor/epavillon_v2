//! La séance suit sa réunion Zoom : `live.meeting.started` la met en direct,
//! `live.meeting.ended` la termine, une relance la rallume. Une séance annulée
//! ou reportée ne bouge pas, et rejouer l'événement ne change rien.

mod commun;

use commun::Bac;
use kernel::context::RequestContext;
use kernel::events::{self, ConsumerRegistry, OutboxEvent};
use uuid::Uuid;

async fn seance(bac: &Bac, event_id: Uuid, slug: &str, statut: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, status, starts_at, ends_at, is_streamed, format, timezone,
                published_at)
           VALUES ($1, '{"fr":"Atelier"}', $2::text::platform.slug,
                   $3::text::programme.session_status,
                   now() - interval '5 minutes', now() + interval '85 minutes', true, 'online',
                   'America/Belem', now())
        RETURNING id"#,
        event_id,
        slug,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("séance")
}

async fn reunion(bac: &Bac, session_id: Uuid, numero: &str) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO live.meetings (session_id, event_id, external_id, status, sync_status)
         SELECT id, event_id, $2, 'scheduled', 'synced' FROM programme.sessions WHERE id = $1
         RETURNING id",
        session_id,
        numero
    )
    .fetch_one(bac.pool())
    .await
    .expect("réunion")
}

/// Ce que ferait Zoom, puis le relais d'outbox de chaque événement émis.
async fn zoom(bac: &Bac, meeting_id: Uuid, statut: &str, occurrence: &str) {
    sqlx::query!(
        "UPDATE live.meetings SET status = $2::text::live.meeting_status, external_uuid = $3
          WHERE id = $1",
        meeting_id,
        statut,
        occurrence
    )
    .execute(bac.pool())
    .await
    .expect("état Zoom");
    relayer(bac, meeting_id).await;
}

async fn relayer(bac: &Bac, meeting_id: Uuid) {
    let registre = ConsumerRegistry::new().register_all(programme::event_consumers());
    let lignes = sqlx::query!(
        "SELECT id, aggregate_schema, aggregate_type, aggregate_id, event_type, event_version,
                payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events WHERE aggregate_id = $1 ORDER BY occurred_at, id",
        meeting_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("outbox");
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .expect("transaction");
    for l in lignes {
        let evenement = OutboxEvent {
            id: l.id,
            aggregate_schema: l.aggregate_schema,
            aggregate_type: l.aggregate_type,
            aggregate_id: l.aggregate_id,
            event_type: l.event_type,
            event_version: l.event_version,
            payload: l.payload,
            metadata: l.metadata,
            correlation_id: l.correlation_id,
            occurred_at: l.occurred_at,
        };
        for c in registre.interested(&evenement.event_type) {
            if events::claim(&mut tx, c.name(), evenement.id)
                .await
                .expect("réservation")
            {
                c.handle(&mut tx, &evenement).await.expect("consommation");
            }
        }
    }
    tx.commit().await.expect("validation");
}

async fn statut(bac: &Bac, session_id: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM programme.sessions WHERE id = $1"#,
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("statut")
}

#[tokio::test]
async fn la_seance_suit_sa_reunion() {
    let bac = Bac::monter().await;
    let edition = commun::edition_cop31(&bac).await;
    let s = seance(&bac, edition, "atelier", "scheduled").await;
    let m = reunion(&bac, s, "85746065432").await;

    zoom(&bac, m, "started", "occ-1").await;
    assert_eq!(statut(&bac, s).await, "live");
    zoom(&bac, m, "ended", "occ-1").await;
    assert_eq!(statut(&bac, s).await, "completed");
    zoom(&bac, m, "started", "occ-2").await;
    assert_eq!(
        statut(&bac, s).await,
        "live",
        "relancée, la séance repasse en direct"
    );

    let changements = sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events WHERE aggregate_id = $1 ORDER BY occurred_at, id",
        s
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        &changements[1..],
        [
            "programme.session.live",
            "programme.session.completed",
            "programme.session.live"
        ],
        "un changement d'état par événement, rejeu compris"
    );
}

#[tokio::test]
async fn une_seance_annulee_ou_reportee_ne_bouge_pas() {
    let bac = Bac::monter().await;
    let edition = commun::edition_cop31(&bac).await;
    for (slug, etat) in [
        ("annulee", "cancelled"),
        ("reportee", "postponed"),
        ("prevue", "planned"),
    ] {
        let s = seance(&bac, edition, slug, etat).await;
        let m = reunion(&bac, s, &format!("9{}", slug.len())).await;
        zoom(&bac, m, "started", "occ-1").await;
        zoom(&bac, m, "ended", "occ-1").await;
        assert_eq!(statut(&bac, s).await, etat);
    }
}
