//! **Le rappel part une fois, et une seule.**
//!
//! # Ce travail est CONSOMMÉ, jamais créé
//!
//! `engagement.schedule_session_reminders()` le met en file avec l'identifiant
//! du rappel pour clé d'unicité — « seconde barrière contre le double envoi »,
//! dit le modèle. Ce fichier n'enfile rien et n'émet rien.
//!
//! # L'ordre du marquage est le cœur de « une fois, et une seule »
//!
//! La file est « au moins une fois » : un worker tué entre l'envoi et son
//! marquage rejoue le travail entier. Marquer **après** l'envoi produirait alors
//! deux courriels — le défaut exact de la v1, que la clé d'unicité du modèle ne
//! rattrape pas, puisqu'elle interdit deux lignes et non deux envois sur la même
//! ligne.
//!
//! La ligne passe donc à « parti » **avant** l'expédition, et l'écriture rend
//! faux si un autre passage l'a déjà prise. Le prix est écrit : une mort brutale
//! entre le marquage et l'envoi perd un courriel. Un rappel manquant se voit et
//! se rejoue ; un rappel envoyé deux fois est irrattrapable. Les échecs
//! **annoncés**, eux, rendent la ligne à la file.
//!
//! # Trois raisons de ne rien envoyer, et chacune écrit son motif
//!
//! Le canal coupé par la personne, l'adresse sur la liste de suppression, le
//! canal sans expédition. **Jamais en silence** (FR-065) : l'organisation qui
//! lit son calendrier a droit à laquelle des trois.
//!
//! La liste de suppression est consultée **deux fois** — ici pour écrire le
//! motif, et dans la garde d'envoi qui écarte le message. Ce n'est pas un
//! doublon : la garde protège **tous** les courriels de la plateforme, y compris
//! ceux des modules livrés qui n'ont pas de ligne de rappel à annoter.

use async_trait::async_trait;
use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::mail::Mailer;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::reminder::{motifs, NotificationChannel};
use crate::repo::{cross, delivery, reminders, templates};
use crate::service::compose::{self, RappelACompose};

/// Le nom de la tâche, **tel que la fonction du modèle l'écrit**. Toute
/// divergence laisserait les travaux sans gestionnaire.
pub const SEND_REMINDER: &str = "engagement.send_reminder";

/// La file, telle que la fonction du modèle la nomme. Le worker n'écoute que
/// les files que ses gestionnaires déclarent : une file inécoutée laisserait les
/// travaux s'empiler sans erreur ni trace — le défaut trouvé en phase 4.
pub const QUEUE: &str = "email";

pub struct SendReminder {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendReminder {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }

    async fn ecarter(&self, reminder_id: Uuid, motif: &str) -> Result<()> {
        let mut tx = self.db.write(&contexte()).await?;
        reminders::marquer_ecarte(&mut tx, reminder_id, motif).await?;
        tx.commit().await
    }
}

fn contexte() -> RequestContext {
    RequestContext::background("engagement.send_reminder")
}

#[async_trait]
impl JobHandler for SendReminder {
    fn task(&self) -> &'static str {
        SEND_REMINDER
    }

    fn queue(&self) -> &'static str {
        QUEUE
    }

    // `carries_secret()` n'est PAS déclaré, et c'est vérifié : la charge utile
    // ne porte que des identifiants — rappel, séance, personne, canal. Un
    // travail mort la garde, et c'est la seule matière de diagnostic d'un envoi
    // qui n'est jamais parti.

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let reminder_id = identifiant(job)?;

        let Some(rappel) = reminders::a_traiter(self.db.pool(), reminder_id).await? else {
            tracing::info!(%reminder_id, "rappel absent : envoi sans effet");
            return Ok(());
        };

        // Annulé, écarté ou déjà parti : le travail arrive après la décision, et
        // ce n'est pas un échec. C'est ce qui rend inutile de décommander les
        // travaux d'une règle qu'on coupe.
        if !matches!(rappel.status.as_str(), "pending" | "queued") {
            tracing::debug!(%reminder_id, etat = %rappel.status, "rappel déjà traité");
            return Ok(());
        }

        if NotificationChannel::from_db(&rappel.channel) != Some(NotificationChannel::Email) {
            // `push` n'a aucune implémentation et `in_app` attend l'écran des
            // notifications : dans les deux cas la ligne le **dit**, plutôt que
            // de rester en attente pour toujours.
            self.ecarter(reminder_id, motifs::CANAL_NON_SERVI).await?;
            return Ok(());
        }

        if !delivery::canal_autorise(
            self.db.pool(),
            rappel.person_id,
            &rappel.type_code,
            &rappel.channel,
        )
        .await?
        {
            self.ecarter(reminder_id, motifs::CANAL_COUPE).await?;
            return Ok(());
        }

        let Some(destinataire) =
            cross::personne_pour_courriel(self.db.pool(), rappel.person_id).await?
        else {
            tracing::warn!(%reminder_id, "destinataire introuvable : rappel écarté");
            self.ecarter(reminder_id, motifs::INSCRIPTION_ANNULEE)
                .await?;
            return Ok(());
        };

        if delivery::adresse_supprimee(self.db.pool(), &destinataire.email).await? {
            self.ecarter(reminder_id, motifs::SUPPRIME).await?;
            return Ok(());
        }

        let Some(seance) =
            cross::seance_pour_rappel(self.db.pool(), rappel.session_id, &destinataire.locale)
                .await?
        else {
            tracing::warn!(%reminder_id, "séance disparue : rappel écarté");
            self.ecarter(reminder_id, motifs::SEANCE_ANNULEE).await?;
            return Ok(());
        };

        let compose = compose::rappel(
            self.db.pool(),
            &RappelACompose {
                message_id: &job.id.to_string(),
                destinataire: &destinataire,
                seance: &seance,
                offset_minutes: rappel.offset_minutes,
                type_code: &rappel.type_code,
                template_id: rappel.template_id,
                app_public_url: &self.app_public_url,
            },
        )
        .await?;

        // **Avant l'envoi.** Faux : un autre passage l'a déjà pris, et ce
        // travail n'a rien à faire.
        let mut tx = self.db.write(&contexte()).await?;
        let pris = reminders::marquer_parti(&mut tx, reminder_id).await?;
        tx.commit().await?;
        if !pris {
            tracing::debug!(%reminder_id, "rappel pris par un autre passage");
            return Ok(());
        }

        let issue = self.mailer.send(&compose.mail).await;

        // **La trace porte le modèle et le numéro de révision réellement
        // servis** (FR-089), qu'ils existent ou non : un `template_id` nul dit
        // que le texte de secours a servi, donc qu'un modèle manque. Elle est
        // annotée après coup, sur le travail qui l'a ouverte — la garde d'envoi
        // écrit la même trace pour les courriels des modules livrés, qui n'ont
        // pas de modèle, et l'enrichir casserait leurs six sites de
        // construction.
        let mut tx = self.db.write(&contexte()).await?;
        templates::annoter_la_trace(
            &mut tx,
            job.id,
            &rappel.type_code,
            compose.template,
            reminder_id,
            rappel.person_id,
        )
        .await?;
        tx.commit().await?;

        match issue {
            Ok(()) => Ok(()),
            Err(erreur) => {
                // Échec **annoncé** : la ligne revient à la file et le travail
                // se reprend, avec le délai croissant de `platform.fail_job()`.
                let mut tx = self.db.write(&contexte()).await?;
                reminders::rendre_a_la_file(&mut tx, reminder_id).await?;
                tx.commit().await?;
                Err(ApiError::internal(erreur.to_string()))
            }
        }
    }
}

fn identifiant(job: &ClaimedJob) -> Result<Uuid> {
    job.payload
        .get("reminder_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|brut| brut.parse().ok())
        .ok_or_else(|| ApiError::internal("charge utile sans « reminder_id »"))
}
