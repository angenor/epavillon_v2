//! `engagement.regie_notice` : les courriels de la régie du jour (02/11).
//!
//! `cancelled` part à l'heure dite (cinq minutes après l'annulation) et
//! vérifie d'abord, sous verrou partagé sur le geste, que l'annulation tient
//! encore : défaite d'ici là, rien ne part. Il pose un courriel par inscrit,
//! clé `regie:<geste>:<personne>`, ce qui dit ensuite à qui on a écrit.
//!
//! `restored` écrit « finalement maintenue » à ceux-là seulement : un courriel
//! parti ne se reprend pas, on envoie le correctif.

use async_trait::async_trait;
use contracts::negotiation::{Notification, NotificationSubject, NotificationText};
use contracts::programme::{regie_email_prefix, REGIE_NOTICE_TASK};
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::jobs::notification_email::{self, NOTIFICATION_EMAIL};
use crate::repo::cross;

pub const ANNULEE: &str = "cancelled";
pub const MAINTENUE: &str = "restored";

const TYPE_ANNULEE: &str = "programme.session.cancelled";
const TYPE_MAINTENUE: &str = "programme.session.restored";

#[derive(Debug, Serialize, Deserialize)]
struct Charge {
    kind: String,
    action_id: Uuid,
    session_id: Uuid,
}

/// Mis en file par le consommateur de la régie, à l'heure dite.
pub async fn poser(
    conn: &mut PgConnection,
    kind: &str,
    action_id: Uuid,
    session_id: Uuid,
    a: OffsetDateTime,
) -> Result<()> {
    let charge = serde_json::to_value(Charge {
        kind: kind.to_owned(),
        action_id,
        session_id,
    })
    .map_err(ApiError::internal)?;
    jobs::enqueue(
        conn,
        NewJob {
            queue: crate::jobs::send_reminder::QUEUE,
            ..NewJob::new(REGIE_NOTICE_TASK, charge)
        }
        .idempotent(contracts::programme::regie_notice_key(kind, action_id))
        .at(a),
    )
    .await?;
    Ok(())
}

pub struct RegieNotice {
    db: Db,
}

impl RegieNotice {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for RegieNotice {
    fn task(&self) -> &'static str {
        REGIE_NOTICE_TASK
    }

    fn queue(&self) -> &'static str {
        crate::jobs::send_reminder::QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let c: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge de la régie illisible : {e}")))?;
        let mut tx = self.db.write(&job.context()).await?;
        let pose = match c.kind.as_str() {
            ANNULEE => annulee(&mut tx, &self.db, &c).await?,
            MAINTENUE => maintenue(&mut tx, &self.db, &c).await?,
            autre => {
                tracing::warn!(kind = %autre, "courriel de régie inconnu");
                0
            }
        };
        tx.commit().await?;
        tracing::info!(action = %c.action_id, kind = %c.kind, courriels = pose, "régie : courriels posés");
        Ok(())
    }
}

async fn annulee(conn: &mut PgConnection, db: &Db, c: &Charge) -> Result<usize> {
    if !cross::annulation_de_regie_tient(&mut *conn, c.action_id).await? {
        return Ok(0);
    }
    let destinataires = cross::inscrits_a_prevenir(db.pool(), c.session_id).await?;
    let Some(n) = notification(db, c, TYPE_ANNULEE, destinataires).await? else {
        return Ok(0);
    };
    notification_email::poser_avec_cle(
        conn,
        regie_email_prefix(c.action_id).trim_end_matches(':'),
        &n,
    )
    .await?;
    Ok(n.recipients.len())
}

async fn maintenue(conn: &mut PgConnection, db: &Db, c: &Charge) -> Result<usize> {
    let destinataires = cross::prevenus_par_la_regie(
        &mut *conn,
        NOTIFICATION_EMAIL,
        &regie_email_prefix(c.action_id),
    )
    .await?;
    let Some(n) = notification(db, c, TYPE_MAINTENUE, destinataires).await? else {
        return Ok(0);
    };
    notification_email::poser_avec_cle(conn, &format!("regie-maintenue:{}", c.action_id), &n)
        .await?;
    Ok(n.recipients.len())
}

async fn notification(
    db: &Db,
    c: &Charge,
    type_code: &str,
    destinataires: Vec<Uuid>,
) -> Result<Option<Notification>> {
    if destinataires.is_empty() {
        return Ok(None);
    }
    let (Some(fr), Some(en)) = (
        cross::seance_pour_rappel(db.pool(), c.session_id, "fr").await?,
        cross::seance_pour_rappel(db.pool(), c.session_id, "en").await?,
    ) else {
        return Ok(None);
    };
    let (title, body) = if type_code == TYPE_ANNULEE {
        (
            NotificationText {
                fr: format!("Activité annulée : {}", fr.titre),
                en: format!("Activity cancelled: {}", en.titre),
            },
            NotificationText {
                fr: format!(
                    "L'activité « {} » prévue le {} ({}) est annulée. Nous vous prions de nous en excuser.",
                    fr.titre, fr.debut_local, fr.timezone
                ),
                en: format!(
                    "The activity \"{}\" scheduled for {} ({}) is cancelled. We apologise for the inconvenience.",
                    en.titre, en.debut_local, en.timezone
                ),
            },
        )
    } else {
        (
            NotificationText {
                fr: format!("Finalement, l'activité est maintenue : {}", fr.titre),
                en: format!("The activity is going ahead after all: {}", en.titre),
            },
            NotificationText {
                fr: format!(
                    "Finalement, l'activité « {} » est maintenue le {} ({}). Ne tenez pas compte du message d'annulation reçu plus tôt.",
                    fr.titre, fr.debut_local, fr.timezone
                ),
                en: format!(
                    "The activity \"{}\" is going ahead after all on {} ({}). Please disregard the cancellation message sent earlier.",
                    en.titre, en.debut_local, en.timezone
                ),
            },
        )
    };
    Ok(Some(Notification {
        type_code: type_code.to_owned(),
        recipients: destinataires,
        title,
        body,
        link_path: format!("/activity/{}/{}", fr.event_slug, fr.slug),
        subject: NotificationSubject {
            schema: "programme".to_owned(),
            table: "sessions".to_owned(),
            id: c.session_id,
        },
        group_key: None,
        replace: false,
        variables: serde_json::json!({ "titre_session": fr.titre }),
    }))
}
