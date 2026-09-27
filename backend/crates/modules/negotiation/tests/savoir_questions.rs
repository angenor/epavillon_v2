//! **Les questions aux experts**, sur base réelle : poser (rejeu par
//! `client_ref`), « Mes questions », répondre — le courriel mis en file une
//! fois, l'événement dans l'outbox —, promouvoir en brouillon sans auteur, et
//! le refus sans consentement (FR-020 à FR-024).

mod commun;

use actix_web::http::StatusCode;
use async_trait::async_trait;
use commun::documents::{expert, negociatrice};
use commun::http::{appel, frapper};
use commun::{courriels_en_file, evenements, Bac};
use kernel::error::ErrorCode;
use kernel::mail::{MailError, Mailer, OutgoingMail};
use negotiation::domain::savoir_questions::MyQuestionInput;
use negotiation::jobs::emails::{mettre_en_file_reponse, SEND_ANSWERED_EMAIL};
use negotiation::service::savoir_questions::{miennes, poser};
use serde_json::json;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct Boite(Mutex<Vec<OutgoingMail>>);

#[async_trait]
impl Mailer for Boite {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.0.lock().expect("boîte").push(mail.clone());
        Ok(())
    }
}

fn question(client_ref: Uuid, body: &str, consentement: bool) -> MyQuestionInput {
    serde_json::from_value(json!({
        "client_ref": client_ref,
        "theme_code": "adaptation",
        "body": body,
        "consent_to_faq": consentement,
    }))
    .expect("question")
}

async fn poser_(bac: &Bac, qui: Uuid, body: &str, consentement: bool) -> Uuid {
    let (q, _) = poser(
        &bac.state,
        &bac.ctx(qui),
        qui,
        &question(Uuid::now_v7(), body, consentement),
        "fr",
    )
    .await
    .expect("question posée");
    q.id
}

#[tokio::test]
async fn une_question_se_pose_une_fois_meme_rejouee() {
    let bac = Bac::monter().await;
    let aissatou = negociatrice(&bac, "aissatou@example.org").await;
    let reference = Uuid::now_v7();
    let entree = question(reference, "  Qui coordonne mon groupe ?  ", true);

    let (premiere, nouvelle) = poser(&bac.state, &bac.ctx(aissatou), aissatou, &entree, "fr")
        .await
        .expect("question");
    assert!(nouvelle);
    assert_eq!(premiere.body, "Qui coordonne mon groupe ?");
    assert_eq!(
        (premiere.status.as_str(), premiere.theme_code.as_str()),
        ("pending", "adaptation")
    );
    assert!(premiere.answer.is_none());

    let (rejouee, nouvelle) = poser(&bac.state, &bac.ctx(aissatou), aissatou, &entree, "fr")
        .await
        .expect("rejeu");
    assert!(!nouvelle);
    assert_eq!(rejouee.id, premiere.id);

    let (liste, empreinte) = miennes(&bac.state, aissatou, "fr").await.expect("liste");
    assert_eq!(liste.questions.len(), 1);
    let (_, meme) = miennes(&bac.state, aissatou, "fr").await.unwrap();
    assert_eq!(empreinte, meme);

    let inconnue: MyQuestionInput = serde_json::from_value(json!({
        "client_ref": Uuid::now_v7(), "theme_code": "astrologie", "body": "Qui ?", "consent_to_faq": true
    }))
    .unwrap();
    let refus = poser(&bac.state, &bac.ctx(aissatou), aissatou, &inconnue, "fr")
        .await
        .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationThemeUnknown);
    let longue = question(Uuid::now_v7(), &"é".repeat(601), true);
    let refus = poser(&bac.state, &bac.ctx(aissatou), aissatou, &longue, "fr")
        .await
        .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationTextTooLong);
}

#[tokio::test]
async fn repondre_met_le_courriel_en_file_une_fois_et_laisse_levenement() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let aissatou = negociatrice(&bac, "aissatou@example.org").await;
    let id = poser_(&bac, aissatou, "Qui coordonne mon groupe ?", true).await;
    let (_, avant) = miennes(&bac.state, aissatou, "fr").await.unwrap();

    let app = crate::back_office!(bac);
    let uri = format!("/admin/negotiation/queue/questions/{id}/answer");
    let (statut, r) = frapper(
        &app,
        appel("post", &uri, Some(experte), Some(json!({ "answer": "  " }))).to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{r}");

    let (statut, r) = frapper(
        &app,
        appel(
            "post",
            &uri,
            Some(experte),
            Some(json!({ "answer": "Le coordinateur du groupe africain." })),
        )
        .to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "answered");
    assert_eq!(r["answer"], "Le coordinateur du groupe africain.");

    let (statut, r) = frapper(
        &app,
        appel(
            "post",
            &uri,
            Some(experte),
            Some(json!({ "answer": "Encore." })),
        )
        .to_request(),
    )
    .await;
    assert_eq!(
        (statut, r["code"].as_str()),
        (StatusCode::CONFLICT, Some("NEGOTIATION_QUEUE_ITEM_CLOSED"))
    );

    let mut conn = bac.pool().acquire().await.unwrap();
    mettre_en_file_reponse(
        &mut conn,
        id,
        "aissatou@example.org",
        "fr",
        "Aïssatou",
        "Qui ?",
    )
    .await
    .expect("seconde mise en file ignorée");
    drop(conn);
    let courriels: Vec<_> = courriels_en_file(&bac)
        .await
        .into_iter()
        .filter(|(tache, _)| tache == SEND_ANSWERED_EMAIL)
        .collect();
    assert_eq!(
        courriels,
        vec![(
            SEND_ANSWERED_EMAIL.to_owned(),
            "aissatou@example.org".to_owned()
        )]
    );
    let boite = Arc::new(Boite::default());
    let gestionnaires = negotiation::job_handlers(bac.db(), &bac.config, boite.clone());
    let gestionnaire = gestionnaires
        .iter()
        .find(|g| g.task() == SEND_ANSWERED_EMAIL)
        .expect("gestionnaire enregistré");
    let mut tx = bac.db().write(&bac.ctx_anonyme()).await.unwrap();
    let travaux = kernel::jobs::claim(&mut tx, gestionnaire.queue(), "test-worker", 10)
        .await
        .expect("réservation dans la file écoutée");
    tx.commit().await.unwrap();
    let travail = travaux
        .iter()
        .find(|t| t.task == SEND_ANSWERED_EMAIL)
        .expect("posé dans la file que le gestionnaire écoute");
    gestionnaire.run(travail).await.expect("envoi");
    let remis = boite.0.lock().unwrap().clone();
    assert_eq!(remis.len(), 1);
    assert_eq!(remis[0].to, "aissatou@example.org");
    assert_eq!(remis[0].subject, "Un expert a répondu à votre question");
    assert!(remis[0].text.contains("Qui coordonne mon groupe ?"));
    assert!(remis[0]
        .text
        .contains("/guide-nego/ressources/faq/mes-questions"));

    assert_eq!(
        evenements(&bac, id).await,
        vec!["negotiation.expert_question.answered".to_owned()]
    );

    let (liste, apres) = miennes(&bac.state, aissatou, "fr").await.unwrap();
    assert_ne!(avant, apres, "l'empreinte suit la réponse");
    let q = &liste.questions[0];
    assert_eq!(q.status, "answered");
    assert!(q.answered_at.is_some() && q.answered_by_name.is_some());
}

#[tokio::test]
async fn promouvoir_donne_un_brouillon_sans_auteur_et_jamais_sans_consentement() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let aissatou = negociatrice(&bac, "aissatou@example.org").await;
    let avec = poser_(&bac, aissatou, "Qui coordonne mon groupe ?", true).await;
    let sans = poser_(&bac, aissatou, "Où dormir à Belém ?", false).await;
    let app = crate::back_office!(bac);
    let promouvoir = |id: Uuid| format!("/admin/negotiation/queue/questions/{id}/promote");
    let rubrique = json!({ "section_code": "first_cop" });

    let (statut, r) = frapper(
        &app,
        appel(
            "post",
            &promouvoir(avec),
            Some(experte),
            Some(rubrique.clone()),
        )
        .to_request(),
    )
    .await;
    assert_eq!(
        statut,
        StatusCode::UNPROCESSABLE_ENTITY,
        "pas encore répondue : {r}"
    );

    for id in [avec, sans] {
        let (statut, _) = frapper(
            &app,
            appel(
                "post",
                &format!("/admin/negotiation/queue/questions/{id}/answer"),
                Some(experte),
                Some(json!({ "answer": "Réponse de l'experte." })),
            )
            .to_request(),
        )
        .await;
        assert_eq!(statut, StatusCode::OK);
    }

    let (statut, r) = frapper(
        &app,
        appel(
            "post",
            &promouvoir(sans),
            Some(experte),
            Some(rubrique.clone()),
        )
        .to_request(),
    )
    .await;
    assert_eq!(
        (statut, r["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("NEGOTIATION_QUESTION_NO_CONSENT")
        )
    );
    let brouillons: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM negotiation.faq_entries WHERE origin_question_id IS NOT NULL",
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(brouillons, 0, "le refus n'a rien laissé");

    let (statut, entree) = frapper(
        &app,
        appel(
            "post",
            &promouvoir(avec),
            Some(experte),
            Some(rubrique.clone()),
        )
        .to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::OK, "{entree}");
    assert_eq!(entree["status"], "draft");
    assert_eq!(entree["origin_question_id"], json!(avec));
    assert_eq!(entree["section_code"], "first_cop");
    assert_eq!(entree["question"]["fr"], "Qui coordonne mon groupe ?");
    assert_eq!(entree["answer"]["fr"], "Réponse de l'experte.");
    let id: Uuid = serde_json::from_value(entree["id"].clone()).unwrap();
    let (auteur, statut_question): (Option<Uuid>, String) = sqlx::query_as(
        "SELECT f.created_by, q.status::text
           FROM negotiation.faq_entries f
           JOIN negotiation.expert_questions q ON q.faq_entry_id = f.id
          WHERE f.id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!((auteur, statut_question.as_str()), (None, "added_to_faq"));

    let (statut, r) = frapper(
        &app,
        appel("post", &promouvoir(avec), Some(experte), Some(rubrique)).to_request(),
    )
    .await;
    assert_eq!(
        (statut, r["code"].as_str()),
        (StatusCode::CONFLICT, Some("NEGOTIATION_QUEUE_ITEM_CLOSED"))
    );

    let (liste, _) = miennes(&bac.state, aissatou, "fr").await.unwrap();
    let promue = liste.questions.iter().find(|q| q.id == avec).unwrap();
    assert_eq!(promue.status, "added_to_faq");
}
