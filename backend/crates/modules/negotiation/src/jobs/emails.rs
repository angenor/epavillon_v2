//! Les deux envois de courriel du module.
//!
//! **La mise en file se fait dans la transaction de la décision** : si elle est
//! annulée, le courriel ne naît pas. C'est la garantie de FR-028 — rien ne part
//! avant que la décision soit enregistrée —, et elle ne demande aucun autre
//! mécanisme que `jobs::enqueue` sur la même connexion.
//!
//! **Le gestionnaire ne touche pas la base** : tout ce qu'il lui faut est dans
//! la charge utile. L'envoi ne dépend donc jamais d'une lecture qui aurait pu
//! changer depuis la mise en file — une demande retranchée, un espace renommé.
//!
//! **Aucun secret ici**, à la différence de l'invitation d'organisation : ces
//! deux messages ne portent ni jeton ni code, seulement une décision et son
//! motif. `carries_secret()` reste donc au défaut.

use async_trait::async_trait;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Mailer, OutgoingMail};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::mail::{self, MailContext};

pub const SEND_APPROVED_EMAIL: &str = "negotiation.access_request.approved_email";
pub const SEND_REJECTED_EMAIL: &str = "negotiation.access_request.rejected_email";

#[derive(Debug, Deserialize)]
struct Charge {
    to: String,
    locale: String,
    first_name: String,
    #[serde(default)]
    space_name: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

/// Met en file le courriel d'admission. La clé d'unicité porte la demande :
/// une décision ne s'envoie qu'une fois, quel que soit le nombre de reprises.
pub async fn mettre_en_file_admission(
    conn: &mut PgConnection,
    request_id: Uuid,
    email: &str,
    locale: &str,
    first_name: &str,
    space_name: Option<&str>,
) -> Result<()> {
    jobs::enqueue(
        conn,
        NewJob::new(
            SEND_APPROVED_EMAIL,
            json!({
                "to": email,
                "locale": locale,
                "first_name": first_name,
                "space_name": space_name,
            }),
        )
        .idempotent(format!("{SEND_APPROVED_EMAIL}:{request_id}")),
    )
    .await?;

    Ok(())
}

/// Met en file le courriel de refus, **avec son motif** : c'est la seule chose
/// que la personne lira pour comprendre.
pub async fn mettre_en_file_refus(
    conn: &mut PgConnection,
    request_id: Uuid,
    email: &str,
    locale: &str,
    first_name: &str,
    motif: Option<&str>,
) -> Result<()> {
    jobs::enqueue(
        conn,
        NewJob::new(
            SEND_REJECTED_EMAIL,
            json!({
                "to": email,
                "locale": locale,
                "first_name": first_name,
                "reason": motif,
            }),
        )
        .idempotent(format!("{SEND_REJECTED_EMAIL}:{request_id}")),
    )
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------

pub struct SendApprovedEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

pub struct SendRejectedEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendApprovedEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

impl SendRejectedEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

#[async_trait]
impl JobHandler for SendApprovedEmail {
    fn task(&self) -> &'static str {
        SEND_APPROVED_EMAIL
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let identifiant = job.id.to_string();
        let message = mail::demande_admise(&contexte(&identifiant, &charge, &self.app_public_url));
        remettre(self.mailer.as_ref(), &message).await
    }
}

#[async_trait]
impl JobHandler for SendRejectedEmail {
    fn task(&self) -> &'static str {
        SEND_REJECTED_EMAIL
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let identifiant = job.id.to_string();
        let message = mail::demande_refusee(
            &contexte(&identifiant, &charge, &self.app_public_url),
            charge.reason.as_deref(),
        );
        remettre(self.mailer.as_ref(), &message).await
    }
}

fn lire(job: &ClaimedJob) -> Result<Charge> {
    serde_json::from_value(job.payload.clone())
        .map_err(|e| ApiError::internal(format!("charge utile illisible : {e}")))
}

fn contexte<'a>(
    message_id: &'a str,
    charge: &'a Charge,
    app_public_url: &'a str,
) -> MailContext<'a> {
    MailContext {
        message_id,
        to: &charge.to,
        locale: &charge.locale,
        first_name: &charge.first_name,
        space_name: charge.space_name.as_deref(),
        app_public_url,
    }
}

/// L'erreur rendue est **relue par l'exploitation** : elle porte le code stable
/// et le statut, jamais l'adresse ni le corps renvoyé par le relais.
pub(crate) async fn remettre(mailer: &dyn Mailer, message: &OutgoingMail) -> Result<()> {
    use kernel::mail::MailError;

    mailer.send(message).await.map_err(|e| match e {
        MailError::Unreachable(_) => ApiError::internal(format!(
            "{} : relais injoignable",
            kernel::mail::MAIL_RELAY_UNREACHABLE
        )),
        MailError::Rejected { statut, .. } => ApiError::internal(format!(
            "{} : relais refusé ({statut})",
            kernel::mail::MAIL_RELAY_UNREACHABLE
        )),
        MailError::NotWired(raison) => ApiError::internal(raison),
    })
}
