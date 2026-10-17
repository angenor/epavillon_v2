//! La réception d'une notification Zoom : vérifier, journaliser, mettre en
//! file, répondre vite. Le traitement vit dans `jobs::webhook`.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, NewJob};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::repo::meetings::{FILE, PROCESS_WEBHOOK};
use crate::repo::webhooks::{self, NouveauWebhook};
use crate::state::LiveState;
use crate::zoom::webhook as signature;

pub struct EnTetes<'a> {
    pub signature: Option<&'a str>,
    pub horodatage: Option<&'a str>,
}

/// `Some` : la réponse à la validation d'adresse. `None` : notification reçue.
/// Une signature fausse ou périmée est journalisée (jamais appliquée) puis
/// refusée en 401.
pub async fn recevoir(
    state: &LiveState,
    en_tetes: &EnTetes<'_>,
    corps: &[u8],
    maintenant: OffsetDateTime,
) -> Result<Option<Value>> {
    let zoom = state.zoom()?;
    let secret = zoom
        .config
        .webhook_secret_token
        .as_ref()
        .ok_or_else(ApiError::not_found)?;

    let charge: Value = serde_json::from_slice(corps)
        .map_err(|_| ApiError::validation("Notification illisible.", "body"))?;
    let evenement = charge["event"].as_str().unwrap_or_default().to_owned();

    let verifiee = match (en_tetes.signature, en_tetes.horodatage) {
        (Some(s), Some(h)) => {
            signature::signature_valide(secret.expose(), h, corps, s, maintenant.unix_timestamp())
        }
        _ => false,
    };

    // Valider l'adresse sans signature ferait de la route un oracle HMAC : la
    // réponse signerait n'importe quel texte, notifications forgées comprises.
    if evenement == "endpoint.url_validation" {
        if !verifiee {
            return Err(ApiError::unauthenticated());
        }
        let jeton = charge["payload"]["plainToken"].as_str().unwrap_or_default();
        return Ok(Some(signature::reponse_de_validation(
            secret.expose(),
            jeton,
        )));
    }

    let objet = &charge["payload"]["object"];
    let external_id = match &objet["id"] {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    };
    let occurred_at = charge["event_ts"]
        .as_i64()
        .and_then(|ms| OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000).ok());

    let ctx = RequestContext::background("live.webhook");
    let mut tx = state.db().write(&ctx).await?;
    let id = webhooks::journaliser(
        &mut tx,
        &NouveauWebhook {
            event_type: if evenement.is_empty() {
                "inconnu"
            } else {
                &evenement
            },
            payload_hash: Sha256::digest(corps).to_vec(),
            signature: en_tetes.signature,
            signature_verified: verifiee,
            external_id,
            payload: &charge,
            headers: json!({ "x-zm-request-timestamp": en_tetes.horodatage }),
            occurred_at,
        },
    )
    .await?;
    if let (Some(id), true) = (id, verifiee) {
        let mut job = NewJob::new(PROCESS_WEBHOOK, json!({ "webhook_event_id": id }))
            .idempotent(id.to_string());
        job.queue = FILE;
        jobs::enqueue(&mut tx, job).await?;
    }
    tx.commit().await?;

    if verifiee {
        Ok(None)
    } else {
        Err(ApiError::unauthenticated())
    }
}
