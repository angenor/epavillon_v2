//! Le décor des tests Zoom : un Zoom configuré à la main, des inscriptions, et
//! le passage d'une inscription par le consommateur.

use kernel::config::{Secret, ZoomConfig};
use kernel::context::RequestContext;
use kernel::events::{EventConsumer, OutboxEvent};
use live::consumers::registrations::RegistrationsConsumer;
use live::state::LiveState;
use live::zoom::Zoom;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::Bac;

pub fn reglages() -> ZoomConfig {
    ZoomConfig {
        account_id: "compte".into(),
        client_id: "client".into(),
        client_secret: Secret::from("secret".to_owned()),
        host_user: "me".into(),
        sdk_key: "cle-sdk".into(),
        sdk_secret: Secret::from("secret-sdk".to_owned()),
        webhook_secret_token: Some(Secret::from("jeton-zoom".to_owned())),
        secrets_key: Secret::from("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=".to_owned()),
        // Rien n'écoute ici : un test qui appellerait Zoom échouerait.
        api_base_url: "http://127.0.0.1:9/v2".into(),
        oauth_url: "http://127.0.0.1:9/oauth/token".into(),
    }
}

pub fn etat_avec_zoom(bac: &Bac) -> LiveState {
    bac.state
        .clone()
        .avec_zoom(Zoom::depuis(&reglages()).expect("Zoom de test"))
}

/// Une inscription posée en base ; le déclencheur du modèle la bascule en
/// liste d'attente si la jauge est pleine.
pub async fn inscrire(bac: &Bac, session_id: Uuid, person_id: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO programme.registrations (session_id, person_id) VALUES ($1, $2) RETURNING id",
        session_id,
        person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("inscription")
}

pub async fn annuler(bac: &Bac, registration_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.registrations
            SET status = 'cancelled', waitlist_position = NULL, cancelled_at = now()
          WHERE id = $1",
        registration_id
    )
    .execute(bac.pool())
    .await
    .expect("annulation");
}

pub async fn statut(bac: &Bac, registration_id: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM programme.registrations WHERE id = $1"#,
        registration_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("statut")
}

/// Ce que le relais d'outbox ferait de l'événement de cette inscription.
pub async fn consommer(bac: &Bac, registration_id: Uuid) {
    let evenement = OutboxEvent {
        id: Uuid::now_v7(),
        aggregate_schema: "programme".into(),
        aggregate_type: "registration".into(),
        aggregate_id: registration_id,
        event_type: "programme.registration.created".into(),
        event_version: 1,
        payload: json!({}),
        metadata: json!({}),
        correlation_id: None,
        occurred_at: OffsetDateTime::now_utc(),
    };
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .expect("transaction");
    RegistrationsConsumer
        .handle(&mut tx, &evenement)
        .await
        .expect("consommation");
    tx.commit().await.expect("validation");
}

pub async fn travaux(bac: &Bac, tache: &str, cle: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = $1 AND queue = 'live' AND payload::text LIKE '%' || $2 || '%'"#,
        tache,
        cle
    )
    .fetch_one(bac.pool())
    .await
    .expect("travaux")
}

pub async fn publier(bac: &Bac, session_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now() WHERE id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .expect("publication");
}
