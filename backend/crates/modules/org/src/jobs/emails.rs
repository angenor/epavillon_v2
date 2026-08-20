//! Les trois travaux d'envoi de courriel du module.
//!
//! **L'invitation porte un secret** : sa charge utile transporte le jeton en
//! clair, que la base ne connaît que haché. Deux conséquences, toutes deux dans
//! le code — la réussite vide la charge utile (`kernel::jobs::succeed`), et
//! l'échec définitif la vide aussi, par `carries_secret()`.
//!
//! **L'identifiant du message se réserve AVANT l'envoi, pas après.** C'est le
//! piège que B1 a payé : le doublon réel est concurrent, pas séquentiel. Ici la
//! clé d'unicité de la file est posée dans la transaction du changement d'état,
//! et l'identifiant du travail sert d'identifiant de message — le site retient
//! quelques minutes ceux qu'il a déjà envoyés, ce qui absorbe une reprise après
//! délai d'attente dépassé.
//!
//! **Le gestionnaire ne touche pas la base** : tout ce qu'il lui faut est dans
//! la charge utile. L'envoi ne dépend donc jamais d'une lecture qui aurait pu
//! changer depuis la mise en file.

use async_trait::async_trait;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Mailer, OutgoingMail};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{MembershipId, OrganizationId};
use crate::mail::{self, MailContext};

pub const SEND_INVITATION_EMAIL: &str = "org.membership.invitation_email";
pub const SEND_REQUEST_EMAIL: &str = "org.membership.request_email";
pub const SEND_APPROVED_EMAIL: &str = "org.membership.approved_email";

/// Ce que la mise en file dépose. `token` n'est présent que pour l'invitation :
/// les deux autres n'en ont pas, et ne doivent pas en avoir.
#[derive(Debug, Deserialize)]
struct Charge {
    to: String,
    locale: String,
    first_name: String,
    organization_name: String,
    #[serde(default)]
    token: Option<String>,
    /// Qui demande — seulement pour le message adressé aux référents.
    #[serde(default)]
    requester_name: Option<String>,
}

/// Met l'invitation en file, **dans la transaction du changement d'état**. Le
/// jeton en clair ne vit que là : ni l'outbox, ni l'événement de domaine ne le
/// voient.
#[allow(clippy::too_many_arguments)]
pub async fn mettre_en_file_invitation(
    conn: &mut PgConnection,
    membership_id: MembershipId,
    email: &str,
    locale: &str,
    first_name: &str,
    organization_name: &str,
    jeton: &str,
) -> Result<()> {
    jobs::enqueue(
        conn,
        NewJob::new(
            SEND_INVITATION_EMAIL,
            json!({
                "to": email,
                "locale": locale,
                "first_name": first_name,
                "organization_name": organization_name,
                "token": jeton,
            }),
        )
        .idempotent(cle(SEND_INVITATION_EMAIL, membership_id)),
    )
    .await?;

    Ok(())
}

/// Prévient les référents de l'organisation qu'une demande les attend.
///
/// Un travail **par référent** : la clé porte l'adhésion, l'organisation et le
/// jour, si bien qu'une même demande ne produit qu'un message par personne et
/// par jour, quel que soit le nombre de reprises.
pub async fn mettre_en_file_demande(
    conn: &mut PgConnection,
    membership_id: MembershipId,
    organization_id: OrganizationId,
) -> Result<()> {
    let destinataires = sqlx::query!(
        r#"SELECT p.primary_email::text AS "email!", p.preferred_locale, p.first_name,
                  o.legal_name,
                  (SELECT d.display_name FROM identity.people d
                    JOIN org.memberships dm ON dm.person_id = d.id
                   WHERE dm.id = $1) AS demandeur
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
             JOIN org.organizations o ON o.id = m.organization_id
            WHERE m.organization_id = $2 AND m.role = 'manager' AND m.status = 'active'"#,
        membership_id.as_uuid(),
        organization_id.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    let jour = OffsetDateTime::now_utc().date();

    for r in destinataires {
        jobs::enqueue(
            &mut *conn,
            NewJob::new(
                SEND_REQUEST_EMAIL,
                json!({
                    "to": r.email,
                    "locale": r.preferred_locale,
                    "first_name": r.first_name,
                    "organization_name": r.legal_name,
                    "requester_name": r.demandeur,
                }),
            )
            .idempotent(format!(
                "{SEND_REQUEST_EMAIL}:{membership_id}:{}:{jour}",
                r.email
            )),
        )
        .await?;
    }

    Ok(())
}

/// Prévient la personne que son adhésion est active.
pub async fn mettre_en_file_approbation(
    conn: &mut PgConnection,
    membership_id: MembershipId,
    person_id: Uuid,
    organization_id: OrganizationId,
) -> Result<()> {
    let ligne = sqlx::query!(
        r#"SELECT p.primary_email::text AS "email!", p.preferred_locale, p.first_name,
                  o.legal_name
             FROM identity.people p
             CROSS JOIN org.organizations o
            WHERE p.id = $1 AND o.id = $2"#,
        person_id,
        organization_id.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await?;

    let Some(r) = ligne else {
        return Ok(());
    };

    let jour = OffsetDateTime::now_utc().date();

    jobs::enqueue(
        conn,
        NewJob::new(
            SEND_APPROVED_EMAIL,
            json!({
                "to": r.email,
                "locale": r.preferred_locale,
                "first_name": r.first_name,
                "organization_name": r.legal_name,
            }),
        )
        .idempotent(format!("{SEND_APPROVED_EMAIL}:{membership_id}:{jour}")),
    )
    .await?;

    Ok(())
}

/// La clé d'unicité d'un envoi lié à une adhésion, portée par l'adhésion et le
/// jour : réessayer dix fois dans l'après-midi n'inonde pas une boîte.
fn cle(tache: &str, membership_id: MembershipId) -> String {
    format!(
        "{tache}:{membership_id}:{}",
        OffsetDateTime::now_utc().date()
    )
}

// -----------------------------------------------------------------------------

pub struct SendInvitationEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

pub struct SendRequestEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

pub struct SendApprovedEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendInvitationEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

impl SendRequestEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

impl SendApprovedEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

#[async_trait]
impl JobHandler for SendInvitationEmail {
    fn task(&self) -> &'static str {
        SEND_INVITATION_EMAIL
    }

    /// Le lien d'invitation fait entrer qui le tient dans une organisation : un
    /// travail mort qui le garderait en clair serait un secret durable dans une
    /// table qu'on relit.
    fn carries_secret(&self) -> bool {
        true
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let jeton = charge
            .token
            .as_deref()
            .ok_or_else(|| ApiError::internal("travail d'invitation sans jeton"))?;

        let identifiant = job.id.to_string();
        let message = mail::invitation(
            &contexte(&identifiant, &charge, &self.app_public_url),
            jeton,
        );
        remettre(self.mailer.as_ref(), &message).await
    }
}

#[async_trait]
impl JobHandler for SendRequestEmail {
    fn task(&self) -> &'static str {
        SEND_REQUEST_EMAIL
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let identifiant = job.id.to_string();
        let demandeur = charge.requester_name.clone().unwrap_or_default();
        let message = mail::demande_recue(
            &contexte(&identifiant, &charge, &self.app_public_url),
            &demandeur,
        );
        remettre(self.mailer.as_ref(), &message).await
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
        let message =
            mail::adhesion_approuvee(&contexte(&identifiant, &charge, &self.app_public_url));
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
        organization_name: &charge.organization_name,
        app_public_url,
    }
}

/// L'erreur rendue est **relue par l'exploitation** : elle porte le code stable
/// et le statut, jamais l'adresse ni le corps renvoyé par le relais.
async fn remettre(mailer: &dyn Mailer, message: &OutgoingMail) -> Result<()> {
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
