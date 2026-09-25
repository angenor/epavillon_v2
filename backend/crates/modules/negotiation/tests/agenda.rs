//! **« Mon agenda »** — ajout et retrait idempotents, `404` sur une session
//! inconnue ou non importée, `409` sur une annulée absente, rappel désarmé sur
//! une annulée, et jamais un refus pour chevauchement (règle n° 2).

mod importation;

use importation::Bac;
use kernel::context::RequestContext;
use kernel::error::{ErrorCode, Result};
use negotiation::domain::agenda::MyAgenda;
use negotiation::service::agenda as service;
use negotiation::state::NegotiationState;
use std::sync::Arc;
use uuid::Uuid;

struct Decor {
    bac: Bac,
    etat: NegotiationState,
    awa: Uuid,
}

async fn monter() -> Decor {
    let bac = Bac::monter().await;
    bac.lire().await;
    let etat = NegotiationState::new(
        bac.base.db(),
        Arc::new(kernel::testing::test_config(bac.base.url())),
    );
    let awa = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ('awa.diallo@example.org', 'Awa', 'Diallo', now()) RETURNING id",
    )
    .fetch_one(bac.pool())
    .await
    .expect("personne");
    Decor { bac, etat, awa }
}

impl Decor {
    fn ctx(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr").with_actor(self.awa)
    }

    async fn id(&self, cle: &str) -> Uuid {
        self.bac.session(cle).await.expect("session").id
    }

    async fn garder(&self, cle: &str, rappel: bool) -> Result<()> {
        let id = self.id(cle).await;
        service::poser(&self.etat, &self.ctx(), self.awa, id, rappel).await
    }

    async fn retirer(&self, id: Uuid) {
        service::retirer(&self.etat, &self.ctx(), self.awa, id)
            .await
            .expect("retrait");
    }

    async fn agenda(&self) -> MyAgenda {
        service::mon_agenda(&self.etat, self.awa)
            .await
            .expect("agenda")
    }

    async fn rappel(&self, cle: &str) -> Option<bool> {
        let id = self.id(cle).await;
        self.agenda()
            .await
            .entries
            .iter()
            .find(|e| e.session_id == id)
            .map(|e| e.remind)
    }

    async fn rappel_enregistre(&self, cle: &str) -> bool {
        sqlx::query_scalar(
            "SELECT remind_before IS NOT NULL FROM negotiation.agenda_entries
              WHERE person_id = $1 AND meeting_id = $2",
        )
        .bind(self.awa)
        .bind(self.id(cle).await)
        .fetch_one(self.bac.pool())
        .await
        .expect("ligne d'agenda")
    }

    async fn traces(&self) -> Vec<(String, Option<Uuid>)> {
        sqlx::query_as(
            "SELECT action, actor_id FROM platform.audit_log
              WHERE entity_table = 'agenda_entries' ORDER BY occurred_at",
        )
        .fetch_all(self.bac.pool())
        .await
        .expect("audit")
    }
}

#[tokio::test]
async fn ajout_rejeu_changement_de_rappel_et_retrait_idempotents() {
    let d = monter().await;

    d.garder("654006", false).await.expect("ajout");
    let premier = d.agenda().await;
    d.garder("654006", false).await.expect("rejeu");
    let rejoue = d.agenda().await;
    assert_eq!(rejoue.entries.len(), 1);
    assert_eq!(rejoue.empreinte(), premier.empreinte());
    assert_eq!(d.rappel("654006").await, Some(false));

    d.garder("654006", true).await.expect("rappel armé");
    assert_eq!(d.rappel("654006").await, Some(true));
    assert_ne!(d.agenda().await.empreinte(), premier.empreinte());

    let id = d.id("654006").await;
    d.retirer(id).await;
    d.retirer(id).await;
    d.retirer(Uuid::now_v7()).await;
    assert!(d.agenda().await.entries.is_empty());

    let traces = d.traces().await;
    let actions: Vec<&str> = traces.iter().map(|(a, _)| a.as_str()).collect();
    assert_eq!(
        actions,
        ["insert", "update", "delete"],
        "le rejeu n'écrit rien"
    );
    assert!(traces.iter().all(|(_, acteur)| *acteur == Some(d.awa)));
}

#[tokio::test]
async fn session_inconnue_ou_non_importee_404() {
    let d = monter().await;
    let refus = service::poser(&d.etat, &d.ctx(), d.awa, Uuid::now_v7(), false)
        .await
        .expect_err("inconnue");
    assert_eq!(refus.code, ErrorCode::NegotiationSessionUnknown);
    assert_eq!(refus.code.status().as_u16(), 404);

    let id = d.id("654006").await;
    sqlx::query("UPDATE negotiation.meetings SET source_key = NULL WHERE id = $1")
        .bind(id)
        .execute(d.bac.pool())
        .await
        .expect("réunion saisie à la main");
    let refus = service::poser(&d.etat, &d.ctx(), d.awa, id, false)
        .await
        .expect_err("non importée");
    assert_eq!(refus.code, ErrorCode::NegotiationSessionUnknown);
}

#[tokio::test]
async fn une_annulee_absente_de_l_agenda_ne_s_ajoute_pas() {
    let d = monter().await;
    let refus = d.garder("654417", false).await.expect_err("annulée");
    assert_eq!(refus.code, ErrorCode::NegotiationSessionCancelled);
    assert_eq!(refus.code.status().as_u16(), 409);
    assert!(d.agenda().await.entries.is_empty());
}

#[tokio::test]
async fn l_annulation_par_l_import_desarme_le_rappel_sans_retirer_la_session() {
    let d = monter().await;
    d.garder("654364", true).await.expect("ajout avec rappel");
    assert_eq!(d.rappel("654364").await, Some(true));

    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;
    d.bac.lire().await;
    assert_eq!(
        d.bac.session("654364").await.expect("session").status,
        "cancelled"
    );

    assert_eq!(d.rappel("654364").await, Some(false), "rappel effectif");
    assert!(
        d.rappel_enregistre("654364").await,
        "la ligne n'est pas réécrite"
    );

    d.garder("654364", true)
        .await
        .expect("déjà dans l'agenda : le geste passe");
    assert!(
        !d.rappel_enregistre("654364").await,
        "enregistré désarmé (FR-035)"
    );
    assert_eq!(d.rappel("654364").await, Some(false));
}

#[tokio::test]
async fn deux_sessions_qui_se_chevauchent_se_gardent_toutes_deux() {
    let d = monter().await;
    let (a, b): (String, String) = sqlx::query_as(
        "SELECT a.source_key, b.source_key
           FROM negotiation.meetings a
           JOIN negotiation.meetings b
             ON a.event_id = b.event_id AND a.id < b.id
            AND a.start_at < b.end_at AND b.start_at < a.end_at
          WHERE a.event_id = $1 AND a.status = 'scheduled' AND b.status = 'scheduled'
          LIMIT 1",
    )
    .bind(d.bac.edition)
    .fetch_one(d.bac.pool())
    .await
    .expect("deux sessions qui se chevauchent");

    d.garder(&a, true).await.expect("première");
    d.garder(&b, true).await.expect("seconde, au même moment");
    assert_eq!(d.agenda().await.entries.len(), 2);
}
