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

use async_trait::async_trait;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::mail::{MailError, Mailer, OutgoingMail};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

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

        let resultat = self.interieur.send(mail).await;
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
