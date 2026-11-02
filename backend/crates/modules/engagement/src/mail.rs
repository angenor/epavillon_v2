//! **LE DÉCORATEUR** — la garde d'envoi et le journal, sans toucher aux modules
//! livrés.
//!
//! # Le problème, et pourquoi il ne se règle pas en modifiant les appelants
//!
//! Les six courriels de B1 et B2 appellent `kernel::mail::Mailer` directement,
//! sans garde ni trace. Les réécrire supposerait que `identity` et `org`
//! connaissent `engagement` — **ce que le principe II interdit** (écart n° 133).
//!
//! Le patron retenu est celui que le noyau annonçait lui-même en B1 : *« le jour
//! où l'envoi se réécrit ici, aucun module ne bouge »*. `GardedMailer`
//! **implémente** le contrat et **enveloppe** l'expéditeur réel. Il est composé
//! dans `AppState::new` et dans `worker/main.rs`, et **aucun module livré ne
//! change d'une ligne**.
//!
//! Le point de contrôle qui le prouve : inscrire une adresse sur la liste de
//! suppression, puis provoquer une **invitation d'organisation** (B2) vers cette
//! même adresse. Rien ne doit partir.
//!
//! # Ce qui n'est PAS fait, et qui est dit
//!
//! `OutgoingMail` n'est **pas** enrichie. Y ajouter le type de notification ou
//! la personne casserait les six sites de construction des modules livrés —
//! donc les modifierait, ce que cette décision vise précisément à éviter. Les
//! traces de B1 et B2 portent donc destinataire, langue, sujet et état, **sans
//! type** : le modèle déclare ces colonnes nullables, et c'est exactement pour
//! cela.
//!
//! # Deux pannes, deux choix opposés, et chacun est délibéré
//!
//! - **La consultation de la liste de suppression échoue** → on envoie quand
//!   même. Bloquer tous les courriels de la plateforme sur une panne de lecture
//!   coûterait plus que d'écrire une fois à une adresse morte.
//! - **L'écriture de la trace échoue** → on envoie quand même. Un courriel de
//!   sécurité ne doit pas être retenu parce qu'un journal n'a pas pu s'écrire.
//!
//! Les deux laissent une trace d'erreur dans les journaux du worker. Ce qui ne
//! se produit dans aucun des deux cas : un envoi **silencieusement** perdu.
//!
//! # La copie à l'équipe (02/11/2026)
//!
//! Chaque courriel automatique part en copie visible aux adresses du réglage
//! `engagement.mail.automatic_cc`, sauf ceux de [`SANS_COPIE`]. Un rappel n'est
//! copié qu'une fois par activité (ou journée) et par délai : l'équipe reçoit
//! ce qu'un inscrit reçoit, pas un exemplaire par inscrit. Le travail se
//! retrouve par `message_id`, qui est son identifiant. Une lecture qui échoue
//! fait partir le courriel sans copie, jamais le retenir.

use async_trait::async_trait;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::mail::{Extras, MailError, Mailer, OutgoingMail};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Les travaux dont le courriel ne part jamais en copie : il porte un lien
/// personnel (compte, mot de passe, invitation, direct d'un invité) ou c'est une
/// confirmation d'inscription. Les envois du back-office ont leurs propres copies.
const SANS_COPIE: [&str; 8] = [
    "identity.send_verification_email",
    "identity.send_existing_account_notice",
    "identity.send_password_reset_email",
    "org.membership.invitation_email",
    "engagement.send_registration",
    "engagement.send_day_registration",
    "engagement.send_mailing",
    "engagement.send_mailing_test",
];

/// L'expéditeur enveloppé.
pub struct GardedMailer {
    interieur: Arc<dyn Mailer>,
    db: Db,
    /// Nom écrit dans `email_messages.provider`. Celui de l'expéditeur réel, pas
    /// celui du décorateur : c'est lui qui a parlé au fournisseur.
    fournisseur: String,
}

impl GardedMailer {
    pub fn new(interieur: Arc<dyn Mailer>, db: Db, fournisseur: impl Into<String>) -> Self {
        Self {
            interieur,
            db,
            fournisseur: fournisseur.into(),
        }
    }

    /// Enveloppe l'expéditeur que la configuration a choisi. Faillible depuis le
    /// 01/09 : décrire un serveur de courriel, c'est pouvoir le décrire mal, et
    /// le démarrage est le seul moment où ça se voit.
    pub fn envelopper(
        cfg: &kernel::config::MailConfig,
        db: Db,
    ) -> Result<Arc<dyn Mailer>, kernel::config::ConfigError> {
        let fournisseur = match cfg.transport {
            kernel::config::MailTransport::Relay => "laravel_relay",
            kernel::config::MailTransport::Smtp => "smtp",
        };
        Ok(Arc::new(Self::new(
            kernel::mail::build(cfg)?,
            db,
            fournisseur,
        )))
    }

    async fn est_supprimee(&self, adresse: &str) -> bool {
        let reponse = sqlx::query_scalar!(
            r#"SELECT engagement.is_email_suppressed($1::text::platform.email) AS "supprimee!""#,
            adresse
        )
        .fetch_one(self.db.pool())
        .await;

        match reponse {
            Ok(supprimee) => supprimee,
            Err(e) => {
                tracing::error!(erreur = %e, "liste de suppression illisible : le courriel part sans garde");
                false
            }
        }
    }

    /// Ouvre la trace, à l'état « en file ». Rend son identifiant, ou `None` si
    /// le journal n'a pas pu s'écrire.
    async fn ouvrir_la_trace(&self, mail: &OutgoingMail) -> Option<(Uuid, time::OffsetDateTime)> {
        let job_id = Uuid::parse_str(&mail.message_id).ok();
        let ecriture = async {
            let mut tx = self
                .db
                .write(&RequestContext::background("engagement.mail"))
                .await?;
            let ligne = sqlx::query!(
                r#"INSERT INTO engagement.email_messages
                       (to_email, locale, subject, job_id, provider, status)
                   VALUES ($1::text::platform.email, $2, $3, $4, $5, 'queued')
                   RETURNING id, created_at"#,
                mail.to,
                mail.locale,
                mail.subject,
                job_id,
                self.fournisseur
            )
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok::<_, kernel::error::ApiError>((ligne.id, ligne.created_at))
        }
        .await;

        match ecriture {
            Ok(trace) => Some(trace),
            Err(e) => {
                tracing::error!(erreur = %e, "trace d'expédition non écrite : le courriel part sans journal");
                None
            }
        }
    }

    /// Clôt la trace. La table est partitionnée par mois : la clé primaire est
    /// `(created_at, id)`, et une mise à jour qui n'en porterait que la moitié
    /// balaierait **toutes** les partitions.
    async fn clore_la_trace(
        &self,
        trace: Option<(Uuid, time::OffsetDateTime)>,
        erreur: Option<&str>,
    ) {
        let Some((id, created_at)) = trace else {
            return;
        };
        let ecriture = async {
            let mut tx = self
                .db
                .write(&RequestContext::background("engagement.mail"))
                .await?;
            sqlx::query!(
                "UPDATE engagement.email_messages
                    SET status     = CASE WHEN $3::text IS NULL THEN 'sent'::engagement.email_status
                                          ELSE 'failed'::engagement.email_status END,
                        sent_at    = CASE WHEN $3::text IS NULL THEN now() END,
                        failed_at  = CASE WHEN $3::text IS NOT NULL THEN now() END,
                        last_error = $3,
                        attempts   = attempts + 1
                  WHERE created_at = $1 AND id = $2",
                created_at,
                id,
                erreur
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok::<_, kernel::error::ApiError>(())
        }
        .await;

        if let Err(e) = ecriture {
            tracing::error!(erreur = %e, "trace d'expédition non close");
        }
    }

    /// Les adresses à mettre en copie de ce courriel, d'après son travail.
    async fn copie_automatique(&self, mail: &OutgoingMail) -> Vec<String> {
        let Ok(job_id) = Uuid::parse_str(&mail.message_id) else {
            return vec![];
        };
        match self.lire_la_copie(job_id).await {
            Ok(adresses) => adresses,
            Err(e) => {
                tracing::error!(erreur = %e, "copie automatique illisible : le courriel part sans copie");
                vec![]
            }
        }
    }

    async fn lire_la_copie(&self, job_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let pool = self.db.pool();
        let Some(job) = sqlx::query!(
            "SELECT task, payload FROM platform.jobs WHERE id = $1",
            job_id
        )
        .fetch_optional(pool)
        .await?
        else {
            return Ok(vec![]);
        };
        if SANS_COPIE.contains(&job.task.as_str()) {
            return Ok(vec![]);
        }
        let premier = match job.task.as_str() {
            "engagement.send_reminder" => {
                let rappel = job
                    .payload
                    .get("reminder_id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query_scalar!(
                    r#"SELECT NOT EXISTS (
                           SELECT 1 FROM engagement.scheduled_reminders r
                             JOIN engagement.scheduled_reminders o
                               ON o.session_id = r.session_id AND o.offset_before = r.offset_before
                              AND o.channel = r.channel AND o.id <> r.id
                            WHERE r.id = $1 AND o.status = 'sent') AS "premier!""#,
                    rappel
                )
                .fetch_one(pool)
                .await?
            }
            "engagement.send_day_reminder" => {
                sqlx::query_scalar!(
                    r#"SELECT NOT EXISTS (
                           SELECT 1 FROM platform.jobs j
                             JOIN programme.track_registrations a
                               ON a.id = ($2::jsonb ->> 'track_registration_id')::uuid
                             JOIN programme.track_registrations b
                               ON b.id = (j.payload ->> 'track_registration_id')::uuid
                            WHERE j.task = 'engagement.send_day_reminder' AND j.status = 'succeeded'
                              AND j.id <> $1 AND b.track_id = a.track_id
                              AND j.payload ->> 'offset_minutes' = $2::jsonb ->> 'offset_minutes'
                              AND j.payload ->> 'starts_at' = $2::jsonb ->> 'starts_at') AS "premier!""#,
                    job_id,
                    job.payload
                )
                .fetch_one(pool)
                .await?
            }
            _ => true,
        };
        if !premier {
            return Ok(vec![]);
        }
        let valeur = sqlx::query_scalar!(
            "SELECT value FROM platform.settings WHERE key = 'engagement.mail.automatic_cc'"
        )
        .fetch_optional(pool)
        .await?;
        Ok(valeur
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
            .unwrap_or_default())
    }

    /// Marque la trace « écartée ».
    ///
    /// L'état retenu est `failed` et non `bounced` : le message n'a pas rebondi,
    /// il n'est jamais parti. `last_error` porte le motif, en clair, parce que
    /// c'est la seule chose qui explique à qui relit pourquoi une personne ne
    /// reçoit plus rien.
    async fn clore_ecartee(&self, trace: Option<(Uuid, time::OffsetDateTime)>) {
        self.clore_la_trace(
            trace,
            Some("adresse sur la liste de suppression : envoi écarté"),
        )
        .await;
    }
}

#[async_trait]
impl Mailer for GardedMailer {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.send_with_extras(mail, &Extras::default()).await
    }

    /// La garde et la trace portent sur le destinataire principal. Les copies
    /// (adresses de l'équipe, saisies par l'administrateur) suivent le message.
    async fn send_with_extras(
        &self,
        mail: &OutgoingMail,
        copies: &Extras,
    ) -> Result<(), MailError> {
        let trace = self.ouvrir_la_trace(mail).await;

        if self.est_supprimee(&mail.to).await {
            self.clore_ecartee(trace).await;
            tracing::info!(
                adresse = %empreinte_adresse(&mail.to),
                "envoi écarté : adresse sur la liste de suppression"
            );
            // **Un succès, et non une erreur.** Écarter est le comportement
            // voulu : rendre une erreur ferait mourir le travail, qui se
            // reprendrait cinq fois pour être écarté cinq fois.
            return Ok(());
        }

        let mut copies = copies.clone();
        for adresse in self.copie_automatique(mail).await {
            let deja = |a: &String| a.eq_ignore_ascii_case(&adresse);
            if !adresse.eq_ignore_ascii_case(&mail.to) && !copies.cc.iter().any(deja) {
                copies.cc.push(adresse);
            }
        }
        let resultat = self.interieur.send_with_extras(mail, &copies).await;
        let erreur = resultat.as_ref().err().map(|e| e.to_string());
        self.clore_la_trace(trace, erreur.as_deref()).await;
        resultat
    }
}

/// SHA-256 de l'adresse en minuscules, en hexadécimal.
///
/// Sert aux traces et à la charge utile de `engagement.email.suppressed` :
/// l'outbox est durable, indexée et relayée, et une adresse électronique est une
/// donnée personnelle. Qui détient déjà l'adresse peut vérifier qu'elle est
/// concernée ; personne ne peut la lire.
pub fn empreinte_adresse(adresse: &str) -> String {
    let empreinte = Sha256::digest(adresse.trim().to_lowercase().as_bytes());
    empreinte.iter().map(|o| format!("{o:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lempreinte_ne_depend_ni_de_la_casse_ni_des_espaces() {
        assert_eq!(
            empreinte_adresse("  Awa.SowFall@Roac-Afrique.ORG "),
            empreinte_adresse("awa.sowfall@roac-afrique.org")
        );
    }

    #[test]
    fn lempreinte_ne_laisse_pas_lire_ladresse() {
        let empreinte = empreinte_adresse("awa@example.org");
        assert_eq!(empreinte.len(), 64);
        assert!(!empreinte.contains("awa"));
        assert!(!empreinte.contains("example"));
    }
}
