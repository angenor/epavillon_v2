//! Contexte de requête : identifiant de requête, acteur, locale.
//!
//! L'identifiant de requête relie une page du navigateur, une trace Jaeger et
//! une ligne de `platform.audit_log`. Il est lu de `X-Request-Id` s'il est
//! présent, engendré sinon.

use std::future::Future;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "X-Request-Id";

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub actor_id: Option<Uuid>,
    /// La session par laquelle l'acteur est arrivé.
    ///
    /// Elle vit ici et non dans le module `identity` parce qu'un module qui
    /// n'a pas le droit d'en dépendre en a besoin : l'usage d'un code
    /// d'invitation garde la session d'où il a été saisi, et c'est ce qui
    /// distingue plus tard une entrée faite du téléphone d'une entrée faite du
    /// site. Nulle pour une écriture du worker ou une requête sans session.
    pub session_id: Option<Uuid>,
    pub locale: String,
}

impl RequestContext {
    pub fn new(request_id: impl Into<String>, locale: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            actor_id: None,
            session_id: None,
            locale: locale.into(),
        }
    }

    /// Contexte des écritures du worker : elles portent un identifiant de
    /// requête comme les autres, sans quoi leur trace d'audit serait anonyme.
    pub fn background(task: &str) -> Self {
        Self::new(format!("{task}:{}", Uuid::now_v7()), "fr")
    }

    pub fn with_actor(&self, actor_id: Uuid) -> Self {
        Self {
            actor_id: Some(actor_id),
            ..self.clone()
        }
    }

    /// L'acteur **et** la session par laquelle il est arrivé : ce que pose
    /// l'intergiciel de session.
    pub fn with_session(&self, actor_id: Uuid, session_id: Uuid) -> Self {
        Self {
            actor_id: Some(actor_id),
            session_id: Some(session_id),
            ..self.clone()
        }
    }

    pub fn generated_request_id() -> String {
        Uuid::now_v7().to_string()
    }
}

tokio::task_local! {
    static CURRENT_REQUEST_ID: String;
}

/// Rend l'identifiant de requête visible depuis la construction d'une réponse
/// d'erreur, où l'on n'a plus accès à la requête.
pub async fn scope<F: Future>(request_id: String, future: F) -> F::Output {
    CURRENT_REQUEST_ID.scope(request_id, future).await
}

pub fn current_request_id() -> Option<String> {
    CURRENT_REQUEST_ID.try_with(|id| id.clone()).ok()
}
