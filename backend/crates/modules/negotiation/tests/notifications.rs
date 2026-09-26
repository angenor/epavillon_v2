//! **Notifier** (3b, US4) — `negotiation` calcule qui prévenir et compose
//! l'avis : un changement importé ou un signalement publié émet un événement
//! avec ses destinataires (l'agenda toujours, la thématique seulement allumée)
//! et pose un courriel par personne et par tranche ; l'accord éteint ne coupe
//! que le courriel ; une validation annulée ne produit rien.

mod importation;
#[macro_use]
mod decision;

use async_trait::async_trait;
use decision::{monter, personne, poster, Decor};
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::mail::{MailError, Mailer, OutgoingMail};
use negotiation::jobs::change_email::SessionChangeEmail;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const CHANGEE: &str = "negotiation.meeting.changed";
const COURRIEL: &str = "negotiation.session_change_email";

#[derive(Default)]
struct Boite(Mutex<Vec<OutgoingMail>>);

#[async_trait]
impl Mailer for Boite {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.0.lock().expect("boîte").push(mail.clone());
        Ok(())
    }
}

impl Boite {
    fn messages(&self) -> Vec<OutgoingMail> {
        self.0.lock().expect("boîte").clone()
    }
}

fn route(id: Uuid, geste: &str) -> String {
    format!("/admin/negotiation/reports/{id}/{geste}")
}

/// Les avis émis d'un type, par sujet : (sujet, destinataires, titre FR).
async fn avis(d: &Decor, type_: &str) -> Vec<(Uuid, Vec<Uuid>, String)> {
    d.evenements()
        .await
        .into_iter()
        .filter(|(t, _)| t == type_)
        .map(|(_, p)| {
            let n = &p["notification"];
            (
                serde_json::from_value(n["subject"]["id"].clone()).expect("sujet"),
                serde_json::from_value(n["recipients"].clone()).expect("destinataires"),
                n["title"]["fr"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

async fn courriels_en_file(d: &Decor) -> Vec<(Uuid, Value, String)> {
    sqlx::query_as(
        "SELECT id, payload, idempotency_key FROM platform.jobs
          WHERE task = $1 AND status = 'queued' ORDER BY id",
    )
    .bind(COURRIEL)
    .fetch_all(d.bac.pool())
    .await
    .expect("courriels en file")
}

async fn envoyer(d: &Decor, boite: &Arc<Boite>) {
    let travail = SessionChangeEmail::new(
        d.bac.base.db(),
        boite.clone(),
        "https://app.test".to_owned(),
    );
    for (id, payload, _) in courriels_en_file(d).await {
        travail
            .run(&ClaimedJob {
                id,
                queue: "default".into(),
                task: COURRIEL.into(),
                payload,
                attempts: 1,
                max_attempts: 5,
            })
            .await
            .expect("le courriel part, ou se tait");
    }
}

async fn suivre(d: &Decor, qui: Uuid, theme: &str, notify: bool) {
    sqlx::query(
        "INSERT INTO negotiation.theme_subscriptions (person_id, theme_term_id, notify_changes)
         SELECT $1, id, $3 FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_theme' AND code = $2",
    )
    .bind(qui)
    .bind(theme)
    .bind(notify)
    .execute(d.bac.pool())
    .await
    .expect("suivi");
}

/// La thématique `finance` posée sur le point de l'ordre du jour de la session.
async fn thematiser(d: &Decor, session: Uuid) {
    let fait = sqlx::query(
        "UPDATE negotiation.agenda_items a
            SET theme_term_id = (SELECT id FROM reference.taxonomy_terms
                                  WHERE taxonomy_code = 'negotiation_theme' AND code = 'finance')
           FROM negotiation.meetings m
          WHERE m.id = $1 AND a.id = m.agenda_item_id",
    )
    .bind(session)
    .execute(d.bac.pool())
    .await
    .expect("thématique du point");
    assert_eq!(fait.rows_affected(), 1, "la session a un point");
}

async fn refuser_les_courriels(d: &Decor, qui: Uuid) {
    sqlx::query(
        "INSERT INTO identity.consents (person_id, purpose, is_granted, policy_version)
         VALUES ($1, 'guide_nego_notifications', false, 'test')",
    )
    .bind(qui)
    .execute(d.bac.pool())
    .await
    .expect("accord éteint");
}

#[tokio::test]
async fn un_changement_importe_previent_lagenda_et_la_thematique_allumee() {
    let d = monter().await;
    let avancee = d.session("654214").await;
    let salle = d.session("654010").await;
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", None).await;
    let binta = personne(&d.bac, "binta@example.org", "Binta", None).await;
    let chloe = personne(&d.bac, "chloe@example.org", "Chloé", None).await;
    d.garder(fatou, avancee).await;
    thematiser(&d, salle).await;
    suivre(&d, binta, "finance", true).await;
    suivre(&d, chloe, "finance", false).await;

    d.bac.lire().await;
    assert!(
        avis(&d, CHANGEE).await.is_empty(),
        "une lecture sans changement n'émet rien"
    );

    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;

    let emis = avis(&d, CHANGEE).await;
    assert_eq!(emis.len(), 2, "{emis:?}");
    let de = |id| emis.iter().find(|(s, _, _)| *s == id).expect("avis");
    assert_eq!(de(avancee).1, [fatou]);
    assert!(
        de(avancee).2.starts_with("Déplacée — "),
        "{}",
        de(avancee).2
    );
    assert_eq!(
        de(salle).1,
        [binta],
        "la thématique éteinte ne prévient pas"
    );
    assert!(de(salle).2.starts_with("Salle changée — "));

    let file = courriels_en_file(&d).await;
    assert_eq!(file.len(), 2);
    assert!(file
        .iter()
        .any(|(_, _, cle)| cle.starts_with(&format!("email:{avancee}:"))
            && cle.ends_with(&fatou.to_string())));

    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    let recus = boite.messages();
    assert_eq!(recus.len(), 2);
    let a_fatou = recus
        .iter()
        .find(|m| m.to == "fatou@example.org")
        .expect("courriel de Fatou");
    assert!(
        a_fatou.subject.starts_with("Déplacée — "),
        "{}",
        a_fatou.subject
    );
    assert!(a_fatou.text.contains(&format!(
        "https://app.test/guide-nego/negociations/{avancee}"
    )));
    assert!(a_fatou.text.contains("(Asia/Istanbul)"), "{}", a_fatou.text);
    assert!(a_fatou.text.contains("Selon la source officielle."));
}

#[tokio::test]
async fn deux_changements_dans_la_tranche_un_seul_courriel() {
    let d = monter().await;
    let avancee = d.session("654214").await;
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", None).await;
    d.garder(fatou, avancee).await;

    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;
    d.bac.jeu("cop30/lecture-1").await;
    d.bac.lire().await;

    assert_eq!(
        avis(&d, CHANGEE)
            .await
            .iter()
            .filter(|(s, _, _)| *s == avancee)
            .count(),
        2
    );
    assert_eq!(
        courriels_en_file(&d).await.len(),
        1,
        "une clé par personne et par tranche"
    );

    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    assert_eq!(boite.messages().len(), 1);
}

#[tokio::test]
async fn laccord_eteint_coupe_le_courriel_pas_lavis() {
    let d = monter().await;
    let avancee = d.session("654214").await;
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", None).await;
    d.garder(fatou, avancee).await;
    refuser_les_courriels(&d, fatou).await;

    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;

    assert_eq!(avis(&d, CHANGEE).await[0].1, [fatou], "l'avis part");
    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    assert!(boite.messages().is_empty(), "aucun courriel");
}

#[tokio::test]
async fn la_publication_emet_et_pose_les_courriels() {
    let d = monter().await;
    let app = application!(d);
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", None).await;
    let binta = personne(&d.bac, "binta@example.org", "Binta", None).await;
    d.garder(fatou, d.sessions[0]).await;
    suivre(&d, binta, "finance", true).await;

    let salle = d
        .signaler("venue", d.sessions[0], None, Some("Salle 4"))
        .await;
    let reunion = d.non_annoncee("2026-11-10").await;
    frapper!(&app, poster(d.ifdd, &route(salle, "validate")));
    frapper!(&app, poster(d.ifdd, &route(reunion, "validate")));
    assert!(
        courriels_en_file(&d).await.is_empty(),
        "rien à la validation"
    );
    d.publier_tout().await;

    let publie = avis(&d, "negotiation.report.published").await;
    assert_eq!(publie.len(), 1);
    assert_eq!(
        (publie[0].0, publie[0].1.clone()),
        (d.sessions[0], vec![fatou])
    );
    let reseau = avis(&d, "negotiation.network_meeting.published").await;
    assert_eq!(reseau.len(), 1);
    assert_eq!(reseau[0].1, [binta]);
    let decide = avis(&d, "negotiation.report.decided").await;
    assert!(decide.iter().all(|(_, r, _)| r == &vec![d.awa]));

    assert_eq!(courriels_en_file(&d).await.len(), 2);
    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    let recus = boite.messages();
    let a_fatou = recus
        .iter()
        .find(|m| m.to == "fatou@example.org")
        .expect("Fatou");
    assert!(
        a_fatou.subject.starts_with("Salle changée — "),
        "{}",
        a_fatou.subject
    );
    assert!(
        a_fatou.text.contains("nouvelle salle : Salle 4"),
        "{}",
        a_fatou.text
    );
    assert!(!a_fatou.text.contains("Awa"), "jamais l'autrice");
    let a_binta = recus
        .iter()
        .find(|m| m.to == "binta@example.org")
        .expect("Binta");
    assert!(a_binta.subject.starts_with("Non annoncée — Groupe Afrique"));
    assert!(a_binta.text.contains("/guide-nego/negociations/reseau/"));
}

#[tokio::test]
async fn une_validation_annulee_ne_produit_ni_avis_ni_courriel() {
    let d = monter().await;
    let app = application!(d);
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", None).await;
    d.garder(fatou, d.sessions[0]).await;
    let id = d.signaler("cancelled", d.sessions[0], None, None).await;

    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    d.publier_tout().await;

    assert!(avis(&d, "negotiation.report.published").await.is_empty());
    assert!(courriels_en_file(&d).await.is_empty());
}
