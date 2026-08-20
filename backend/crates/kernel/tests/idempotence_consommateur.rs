//! Un relais relancé sur des événements déjà traités n'en rejoue **aucun**.
//!
//! Le relais d'outbox est « au moins une fois » : un redémarrage entre l'effet
//! et la publication rejoue l'événement, et c'est voulu — perdre un effet serait
//! pire que le tenter deux fois. Ce qui empêche le second passage de produire un
//! second effet est la réservation `(consommateur, événement)` dans
//! `platform.inbox_events`. Sans elle, un `Ctrl-C` mal placé enverrait deux
//! courriels pour une inscription.

use async_trait::async_trait;
use kernel::events::{self, ConsumerRegistry, DomainEvent, EventConsumer, OutboxEvent};
use kernel::testing::TestDb;
use kernel::{Db, RequestContext};
use sqlx::postgres::PgConnection;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Un consommateur qui compte ses effets. Ce n'est pas un double de la base —
/// l'événement, la réservation et la transaction sont réels ; seul l'effet
/// métier est réduit à un compteur, parce que ce qu'on mesure est son NOMBRE.
struct Compteur {
    effets: Arc<AtomicUsize>,
}

#[async_trait]
impl EventConsumer for Compteur {
    fn name(&self) -> &'static str {
        "test.compteur"
    }

    async fn handle(&self, _conn: &mut PgConnection, _event: &OutboxEvent) -> kernel::Result<()> {
        self.effets.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Ce que fait un tour de relais, réduit à ce qui compte : réserver, produire
/// l'effet, marquer publié. La boucle du worker n'est pas atteignable depuis le
/// noyau ; c'est sa garde qui est éprouvée ici, et elle vit bien dans le noyau.
async fn relayer(db: &Db, registre: &ConsumerRegistry, event_id: Uuid) {
    let mut tx = db
        .write(&RequestContext::background("test-outbox"))
        .await
        .expect("transaction");

    let ligne = sqlx::query!(
        "SELECT aggregate_schema, aggregate_type, aggregate_id, event_type, event_version,
                payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events WHERE id = $1",
        event_id
    )
    .fetch_one(&mut *tx)
    .await
    .expect("relecture de l'événement");

    let evenement = OutboxEvent {
        id: event_id,
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

    for consommateur in registre.interested(&evenement.event_type) {
        if events::claim(&mut tx, consommateur.name(), evenement.id)
            .await
            .expect("réservation")
        {
            consommateur
                .handle(&mut tx, &evenement)
                .await
                .expect("effet du consommateur");
        }
    }

    sqlx::query!(
        "UPDATE platform.outbox_events SET published_at = now() WHERE id = $1",
        event_id
    )
    .execute(&mut *tx)
    .await
    .expect("marquage");

    tx.commit().await.expect("validation");
}

#[tokio::test]
async fn un_relais_relance_ne_rejoue_aucun_evenement_deja_traite() {
    let base = TestDb::new().await;
    let db = base.db();

    let effets = Arc::new(AtomicUsize::new(0));
    let registre = ConsumerRegistry::new().register(Compteur {
        effets: effets.clone(),
    });

    let mut tx = db
        .write(&RequestContext::background("test-emission"))
        .await
        .expect("transaction");
    let event_id = events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: "identity",
            aggregate_type: "person",
            aggregate_id: Uuid::now_v7(),
            event_type: "identity.person.registered",
            payload: serde_json::json!({ "preferred_locale": "fr" }),
        },
    )
    .await
    .expect("émission");
    tx.commit().await.expect("validation");

    relayer(&db, &registre, event_id).await;
    assert_eq!(effets.load(Ordering::SeqCst), 1);

    // Le relais repart de zéro — un redémarrage, un balayage qui reprend un
    // événement déjà passé. Aucun effet supplémentaire ne doit naître.
    relayer(&db, &registre, event_id).await;
    relayer(&db, &registre, event_id).await;

    assert_eq!(
        effets.load(Ordering::SeqCst),
        1,
        "trois passages, un seul effet"
    );

    let reservations = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.inbox_events
            WHERE consumer = 'test.compteur' AND event_id = $1"#,
        event_id
    )
    .fetch_one(base.pool())
    .await
    .expect("comptage des réservations");
    assert_eq!(reservations, 1);
}

/// La garde est nominative : deux consommateurs différents traitent **chacun**
/// l'événement. Une garde portée par le seul identifiant d'événement priverait
/// le second de tout effet — et personne ne s'en apercevrait avant la mise en
/// service du consommateur suivant.
#[tokio::test]
async fn deux_consommateurs_distincts_traitent_chacun_levenement() {
    let base = TestDb::new().await;
    let db = base.db();

    let mut tx = db
        .write(&RequestContext::background("test-emission"))
        .await
        .expect("transaction");
    let event_id = events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: "identity",
            aggregate_type: "person",
            aggregate_id: Uuid::now_v7(),
            event_type: "identity.person.registered",
            payload: serde_json::json!({}),
        },
    )
    .await
    .expect("émission");

    assert!(events::claim(&mut tx, "analytics.compteur", event_id)
        .await
        .expect("réservation du premier"));
    assert!(events::claim(&mut tx, "engagement.bienvenue", event_id)
        .await
        .expect("réservation du second"));
    assert!(
        !events::claim(&mut tx, "analytics.compteur", event_id)
            .await
            .expect("seconde réservation du premier"),
        "le même consommateur ne réserve pas deux fois"
    );

    tx.commit().await.expect("validation");
}
