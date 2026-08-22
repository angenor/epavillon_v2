//! **Un événement du domaine devient un avis à l'écran — quand un type le
//! nomme.**
//!
//! # Ce consommateur écoute TOUT, et c'est délibéré
//!
//! `notification_types.code` suit *« la même grammaire que
//! `outbox_events.event_type` »* : le modèle le dit dans son propre commentaire.
//! La correspondance entre un événement et un avis est donc une **donnée**, pas
//! du code — et ajouter une notification reste un INSERT, comme le modèle le
//! promet. Un filtre synchrone exigerait un cache chargé au démarrage, qu'un
//! type ajouté ensuite rendrait faux, en silence.
//!
//! Le coût est assumé : une ligne d'`inbox_events` par événement relayé. C'est
//! déjà le régime du consommateur de télémétrie.
//!
//! # UNE exception à la correspondance par le code, et elle est écrite
//!
//! Le catalogue déclare `programme.registration.confirmed`. **Aucun événement ne
//! porte ce code** — `registration_status` vaut `registered`, jamais
//! `confirmed`, et le déclencheur émet une **création** portant le statut
//! (écart n° 126, encore lui, du côté des avis cette fois). S'en tenir à la
//! correspondance par le code laisserait donc l'inscription confirmée sans avis,
//! sans erreur et sans trace.
//!
//! La table d'exceptions ci-dessous en compte **une seule ligne**, et elle est
//! là pour être relue le jour où le catalogue sera corrigé.
//!
//! # Trois types sont réellement servis ici, et quatorze ne le sont pas
//!
//! Le catalogue ne dit **pas** qui est destinataire ni d'où viennent les
//! variables : cette résolution est du code, type par type. Les trois retenus
//! sont ceux dont toutes les données existent. Prétendre couvrir les dix-huit
//! reviendrait à écrire quatorze résolutions sans destinataire prouvé (R23). Le
//! quatrième avis du jalon — le rappel de séance — ne passe pas par ici : il
//! part du travail différé, mis en file par la fonction du modèle.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::reminder::NotificationChannel;
use crate::repo::{cross, delivery, notifications};

pub struct NotificationsConsumer {
    db: Db,
}

impl NotificationsConsumer {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

/// Le type que le catalogue nomme pour une inscription prise — et qu'aucun
/// événement ne porte.
const INSCRIPTION_CONFIRMEE: &str = "programme.registration.confirmed";

/// Les états d'inscription qui valent confirmation.
const ETATS_CONFIRMES: [&str; 2] = ["registered", "attended"];

#[async_trait]
impl EventConsumer for NotificationsConsumer {
    /// **Inscrit dans `platform.inbox_events`.** Le renommer ferait rejouer
    /// **tout** l'historique du dépôt, ce consommateur écoutant tout.
    fn name(&self) -> &'static str {
        "engagement.notifications"
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let Some(type_code) = self.type_vise(event).await? else {
            return Ok(());
        };
        let Some(avis) = notifications::type_actif(self.db.pool(), &type_code).await? else {
            return Ok(());
        };

        let (destinataires, variables, lien, sujet) = match type_code.as_str() {
            INSCRIPTION_CONFIRMEE => self.inscription(event).await?,
            contracts::programme::SESSION_CANCELLED => self.seance_annulee(event).await?,
            contracts::programme::SESSION_RESCHEDULED => self.seance_deplacee(event).await?,
            // Le type existe au catalogue, mais **personne ne sait encore à qui
            // il s'adresse**. Ne rien faire est la réponse honnête ; inventer un
            // destinataire en serait une fausse.
            _ => {
                tracing::debug!(%type_code, "type déclaré, destinataires non résolus");
                return Ok(());
            }
        };

        for person_id in destinataires {
            // **Le canal se consulte avant d'écrire**, comme avant d'envoyer :
            // un avis coupé ne doit pas s'accumuler dans un fil que personne ne
            // veut lire.
            if !delivery::canal_autorise(
                self.db.pool(),
                person_id,
                &type_code,
                NotificationChannel::InApp.as_str(),
            )
            .await?
            {
                continue;
            }

            notifications::ecrire(
                conn,
                &notifications::NouvelleNotification {
                    person_id,
                    type_code: &type_code,
                    title: avis.label.clone(),
                    body: None,
                    variables: variables.clone(),
                    link_path: lien.clone(),
                    subject_schema: Some("programme"),
                    subject_table: Some("sessions"),
                    subject_id: Some(sujet),
                    // Une clé par (type, séance) : trois changements sur la même
                    // séance forment **une** ligne portant un compte, tant
                    // qu'elle n'est pas lue (FR-092).
                    group_key: Some(format!("{type_code}:{sujet}")),
                },
            )
            .await?;
        }

        Ok(())
    }
}

impl NotificationsConsumer {
    /// Le type d'avis visé par un événement : son code, ou l'exception écrite.
    async fn type_vise(&self, event: &OutboxEvent) -> Result<Option<String>> {
        if event.event_type.starts_with("programme.registration.") {
            let statut = event
                .payload
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            return Ok(ETATS_CONFIRMES
                .contains(&statut)
                .then(|| INSCRIPTION_CONFIRMEE.to_owned()));
        }
        Ok(Some(event.event_type.clone()))
    }

    async fn inscription(
        &self,
        event: &OutboxEvent,
    ) -> Result<(Vec<Uuid>, serde_json::Value, Option<String>, Uuid)> {
        let session_id = uuid_de(event, "session_id").unwrap_or(event.aggregate_id);
        let person_id = uuid_de(event, "person_id");
        let locale = texte_de(event, "locale").unwrap_or_else(|| "fr".to_owned());
        let seance = cross::seance_pour_rappel(self.db.pool(), session_id, &locale).await?;

        let variables = serde_json::json!({
            "titre_session": seance.as_ref().map(|s| s.titre.clone()),
            "date_session": seance
                .as_ref()
                .map(|s| format!("{} ({})", s.debut_local, s.timezone)),
        });

        Ok((
            person_id.into_iter().collect(),
            variables,
            seance.as_ref().map(lien_vers_ledition),
            session_id,
        ))
    }

    async fn seance_annulee(
        &self,
        event: &OutboxEvent,
    ) -> Result<(Vec<Uuid>, serde_json::Value, Option<String>, Uuid)> {
        let session_id = event.aggregate_id;
        let seance = cross::seance_pour_rappel(self.db.pool(), session_id, "fr").await?;
        let motif = cross::motif_dannulation(self.db.pool(), session_id, "fr").await?;

        let variables = serde_json::json!({
            "titre_session": seance.as_ref().map(|s| s.titre.clone()),
            "motif": motif,
        });

        Ok((
            cross::inscrits_a_prevenir(self.db.pool(), session_id).await?,
            variables,
            seance.as_ref().map(lien_vers_ledition),
            session_id,
        ))
    }

    async fn seance_deplacee(
        &self,
        event: &OutboxEvent,
    ) -> Result<(Vec<Uuid>, serde_json::Value, Option<String>, Uuid)> {
        let session_id = event.aggregate_id;
        let seance = cross::seance_pour_rappel(self.db.pool(), session_id, "fr").await?;

        let variables = serde_json::json!({
            "titre_session": seance.as_ref().map(|s| s.titre.clone()),
            "nouvelle_date": seance
                .as_ref()
                .map(|s| format!("{} ({})", s.debut_local, s.timezone)),
        });

        Ok((
            cross::inscrits_a_prevenir(self.db.pool(), session_id).await?,
            variables,
            seance.as_ref().map(lien_vers_ledition),
            session_id,
        ))
    }
}

/// **Un chemin RELATIF, jamais une adresse absolue** (FR-091).
///
/// Les domaines de préproduction ne doivent pas fuiter dans les données : une
/// notification écrite sur une recette et relue en production mènerait ailleurs.
/// La base le vérifie elle-même — `link_path ~ '^/'`.
///
/// Aucune page de séance n'existe encore dans le front (écart n° 138) : le lien
/// mène à l'édition, ancré sur la séance.
fn lien_vers_ledition(seance: &cross::SeancePourRappel) -> String {
    format!("/event/{}#{}", seance.event_slug, seance.slug)
}

fn uuid_de(event: &OutboxEvent, cle: &str) -> Option<Uuid> {
    event.payload.get(cle)?.as_str()?.parse().ok()
}

fn texte_de(event: &OutboxEvent, cle: &str) -> Option<String> {
    Some(event.payload.get(cle)?.as_str()?.to_owned())
}
