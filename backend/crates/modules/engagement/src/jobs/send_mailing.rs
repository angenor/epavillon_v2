//! **L'envoi d'un courriel du back-office, par lots et sans limite d'envoi.**
//!
//! # Un lot, puis le suivant aussitôt
//!
//! Aucune limite d'envoi (décision du commanditaire, 28/10). Chaque passage
//! envoie au plus [`LOT`] messages et pose aussitôt le passage suivant : un
//! envoi de mille adresses ne tient pas un ouvrier du worker d'un seul tenant,
//! et les rappels et confirmations de la même file passent entre deux lots.
//!
//! # Une fois, et une seule
//!
//! Le destinataire passe à `sending` avant l'expédition (même parti que les
//! rappels) : un travail rejoué ne le reprend pas. La clé d'unicité du passage
//! suivant porte le prochain destinataire en file, ce qui empêche deux chaînes
//! de passages de courir ensemble.
//!
//! # Serveur injoignable : on attend, on ne marque pas en échec
//!
//! Le destinataire revient en file et le travail échoue : `fail_job()` le
//! reprend avec un délai croissant. Un refus du serveur (adresse rejetée) est,
//! lui, définitif pour ce destinataire, et l'envoi continue.

use async_trait::async_trait;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Attachment, Extras, MailError, Mailer, OutgoingMail};
use kernel::storage::Entrepots;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::gabarits::{mailto_reponse, mettre_en_page, MiseEnPage};
use crate::domain::mailing::{self, echapper_html, texte_brut};
use crate::domain::render;
use crate::repo::mailings;

pub const SEND_MAILING: &str = "engagement.send_mailing";
pub const SEND_MAILING_TEST: &str = "engagement.send_mailing_test";
pub const QUEUE: &str = crate::jobs::send_reminder::QUEUE;

/// Messages par passage. Petit : un passage tient un ouvrier du worker, et les
/// rappels et confirmations passent par la même file.
const LOT: usize = 10;

fn contexte() -> RequestContext {
    RequestContext::background("engagement.send_mailing")
}

fn mailing_id(job: &ClaimedJob) -> Result<Uuid> {
    job.payload
        .get("mailing_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::internal("charge sans mailing_id"))
}

/// Le message d'un destinataire : variables brutes dans l'objet, échappées dans
/// le corps HTML, gabarit de l'envoi appliqué. L'aperçu de l'écran passe par
/// ici aussi : ce qu'on voit est ce qui part.
pub fn composer(
    message_id: String,
    to: &str,
    envoi: &mailings::LigneEnvoi,
    valeurs: &HashMap<String, String>,
    site: &str,
) -> Result<OutgoingMail> {
    let echappees: HashMap<String, String> = valeurs
        .iter()
        .map(|(k, v)| (k.clone(), echapper_html(v)))
        .collect();
    let sujet =
        render::rendre(&envoi.subject, valeurs).map_err(|e| ApiError::internal(e.to_string()))?;
    let corps = render::rendre(&envoi.body_html, &echappees)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let h = &envoi.habillage;
    let action = match (&h.cta_label, &h.cta_url) {
        (Some(t), Some(l)) => Some((t.as_str(), l.as_str())),
        _ => None,
    };
    let repondre = if h.reply_button {
        mailto_reponse(envoi.reply_to.as_deref(), &envoi.cc, &sujet)
    } else {
        None
    };
    let html = mettre_en_page(
        &corps,
        &MiseEnPage {
            gabarit: &h.template,
            sujet: &sujet,
            action,
            repondre,
            site,
        },
    );
    let mut texte = texte_brut(&corps);
    if let Some((t, l)) = action {
        texte.push_str(&format!("\n\n{t} : {l}"));
    }
    texte.push_str("\n\nL'équipe de l'ePavillon\nInstitut de la Francophonie pour le développement durable (IFDD)");
    Ok(OutgoingMail {
        message_id,
        to: to.to_owned(),
        locale: "fr".to_owned(),
        subject: sujet,
        text: texte,
        html: Some(html),
    })
}

/// Les pièces jointes d'un envoi, lues sur le stockage (bucket privé) une fois
/// par passage : chaque destinataire reçoit les mêmes octets. Une pièce
/// illisible fait échouer le passage, qui sera repris : partir sans elle serait
/// un autre message que celui qu'on a vérifié.
async fn lire_les_pieces(
    db: &Db,
    entrepots: &Entrepots,
    mailing_id: Uuid,
) -> Result<Vec<Attachment>> {
    let mut pieces = Vec::new();
    for p in mailings::pieces_jointes(db.pool(), mailing_id).await? {
        let contenu = entrepots
            .du_bucket(&p.bucket)
            .get(&p.object_key)
            .await
            .map_err(|e| {
                ApiError::internal(format!(
                    "pièce jointe « {} » illisible : {e}",
                    p.fiche.filename
                ))
            })?;
        pieces.push(Attachment {
            filename: p.fiche.filename,
            mime_type: p.fiche.mime_type,
            content: contenu,
        });
    }
    Ok(pieces)
}

pub struct SendMailing {
    db: Db,
    mailer: Arc<dyn Mailer>,
    entrepots: Entrepots,
    /// L'adresse publique du site : le logo des gabarits.
    site: String,
}

impl SendMailing {
    pub fn new(db: Db, mailer: Arc<dyn Mailer>, entrepots: Entrepots, site: String) -> Self {
        Self {
            db,
            mailer,
            entrepots,
            site,
        }
    }

    async fn clore(&self, id: Uuid, statut: &str, erreur: Option<&str>) -> Result<()> {
        let mut tx = self.db.write(&contexte()).await?;
        mailings::clore(&mut tx, id, statut, erreur).await?;
        tx.commit().await
    }

    async fn est_supprimee(&self, adresse: &str) -> Result<bool> {
        let supprimee = sqlx::query_scalar!(
            r#"SELECT engagement.is_email_suppressed($1::text::platform.email) AS "s!""#,
            adresse
        )
        .fetch_one(self.db.pool())
        .await?;
        Ok(supprimee)
    }
}

#[async_trait]
impl JobHandler for SendMailing {
    fn task(&self) -> &'static str {
        SEND_MAILING
    }

    fn queue(&self) -> &'static str {
        QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let id = mailing_id(job)?;
        let Some(envoi) = mailings::lire(self.db.pool(), id).await? else {
            return Ok(());
        };
        // Arrêté ou déjà clos : le passage arrive après la décision.
        if envoi.status != "queued" {
            return Ok(());
        }

        let copies = Extras {
            cc: envoi.cc.clone(),
            bcc: envoi.bcc.clone(),
            reply_to: envoi.reply_to.clone(),
            attachments: lire_les_pieces(&self.db, &self.entrepots, id).await?,
        };

        for _ in 0..LOT {
            let pris = {
                let mut tx = self.db.write(&contexte()).await?;
                let pris = mailings::prendre(&mut tx, id).await?;
                tx.commit().await?;
                pris
            };
            let Some(pris) = pris else { break };

            if self.est_supprimee(&pris.email).await? {
                self.clore(
                    pris.id,
                    "suppressed",
                    Some("adresse sur la liste de suppression"),
                )
                .await?;
                continue;
            }

            let valeurs = mailing::valeurs(&pris.variables, &envoi.habillage.variable_fallbacks);
            let message = match composer(
                pris.id.to_string(),
                &pris.email,
                &envoi,
                &valeurs,
                &self.site,
            ) {
                Ok(m) => m,
                Err(e) => {
                    self.clore(pris.id, "failed", Some(&e.to_string())).await?;
                    continue;
                }
            };

            match self.mailer.send_with_extras(&message, &copies).await {
                Ok(()) => self.clore(pris.id, "sent", None).await?,
                Err(MailError::Unreachable(detail)) => {
                    self.clore(pris.id, "queued", None).await?;
                    return Err(ApiError::internal(format!(
                        "serveur d'envoi injoignable, reprise plus tard : {detail}"
                    )));
                }
                Err(e) => self.clore(pris.id, "failed", Some(&e.to_string())).await?,
            }
        }

        let mut tx = self.db.write(&contexte()).await?;
        if !mailings::clore_si_fini(&mut tx, id).await? {
            let suivant = sqlx::query_scalar!(
                r#"SELECT min(id::text) AS suivant FROM engagement.mailing_recipients
                    WHERE mailing_id = $1 AND status = 'queued'"#,
                id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or_default();
            jobs::enqueue(
                &mut tx,
                NewJob {
                    queue: QUEUE,
                    task: SEND_MAILING,
                    payload: serde_json::json!({ "mailing_id": id }),
                    idempotency_key: Some(format!("mailing:{id}:{suivant}")),
                    run_at: None,
                },
            )
            .await?;
        }
        tx.commit().await
    }
}

/// L'essai : à la personne qui écrit, sans copie, avec les pièces jointes, objet préfixé.
pub struct SendMailingTest {
    db: Db,
    mailer: Arc<dyn Mailer>,
    entrepots: Entrepots,
    /// L'adresse publique du site : le logo des gabarits.
    site: String,
}

impl SendMailingTest {
    pub fn new(db: Db, mailer: Arc<dyn Mailer>, entrepots: Entrepots, site: String) -> Self {
        Self {
            db,
            mailer,
            entrepots,
            site,
        }
    }
}

#[async_trait]
impl JobHandler for SendMailingTest {
    fn task(&self) -> &'static str {
        SEND_MAILING_TEST
    }

    fn queue(&self) -> &'static str {
        QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let id = mailing_id(job)?;
        let personne = job
            .payload
            .get("person_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| ApiError::internal("charge sans person_id"))?;

        let Some(envoi) = mailings::lire(self.db.pool(), id).await? else {
            return Ok(());
        };
        let Some((adresse, prenom, nom, organisation)) =
            mailings::adresse_de(self.db.pool(), personne).await?
        else {
            return Ok(());
        };
        let edition = mailings::titre_edition(self.db.pool(), envoi.event_id).await?;
        let valeurs = mailing::valeurs(
            &serde_json::json!({
                "prenom": prenom,
                "nom": nom,
                "organisation": organisation.unwrap_or_default(),
                "email": adresse,
                "edition": edition,
            }),
            &envoi.habillage.variable_fallbacks,
        );

        let mut message = composer(job.id.to_string(), &adresse, &envoi, &valeurs, &self.site)?;
        message.subject = format!("[Essai] {}", message.subject);
        let copies = Extras {
            reply_to: envoi.reply_to.clone(),
            attachments: lire_les_pieces(&self.db, &self.entrepots, id).await?,
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&message, &copies)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envoi(sujet: &str, corps: &str) -> mailings::LigneEnvoi {
        let maintenant = time::OffsetDateTime::now_utc();
        mailings::LigneEnvoi {
            id: Uuid::nil(),
            event_id: Uuid::nil(),
            status: "draft".into(),
            subject: sujet.into(),
            body_html: corps.into(),
            audience: vec![],
            extra_addresses: vec![],
            cc: vec!["equipe@ifdd.org".into()],
            bcc: vec![],
            reply_to: None,
            duplicated_from: None,
            created_by_name: None,
            queued_by_name: None,
            queued_at: None,
            completed_at: None,
            cancelled_at: None,
            created_at: maintenant,
            updated_at: maintenant,
            habillage: mailings::Habillage {
                template: "sobre".into(),
                variable_fallbacks: serde_json::json!({ "prenom": "Madame, Monsieur" }),
                reply_button: true,
                cta_label: None,
                cta_url: None,
            },
        }
    }

    #[test]
    fn le_corps_echappe_et_lobjet_non() {
        let e = envoi("Pour {{prenom}}", "<p>Bonjour {{prenom}} {{nom}}</p>");
        let valeurs = mailing::valeurs(
            &serde_json::json!({ "prenom": "A<b>", "nom": "O'Neil" }),
            &e.habillage.variable_fallbacks,
        );
        let m = composer(
            "x".into(),
            "a@example.org",
            &e,
            &valeurs,
            "https://site.org",
        )
        .unwrap();
        assert_eq!(m.subject, "Pour A<b>");
        let html = m.html.unwrap();
        assert!(html.contains("Bonjour A&lt;b&gt; O&#39;Neil"), "{html}");
        assert!(html.contains("mailto:equipe@ifdd.org"), "{html}");
        assert!(m.text.starts_with("Bonjour A<b> O'Neil"), "{}", m.text);
    }

    #[test]
    fn sans_compte_la_valeur_de_secours_sert() {
        let e = envoi("Objet", "<p>Bonjour {{prenom}},</p>");
        let valeurs = mailing::valeurs(&serde_json::json!({}), &e.habillage.variable_fallbacks);
        let m = composer(
            "x".into(),
            "a@example.org",
            &e,
            &valeurs,
            "https://site.org",
        )
        .unwrap();
        assert!(
            m.text.starts_with("Bonjour Madame, Monsieur,"),
            "{}",
            m.text
        );
    }
}
