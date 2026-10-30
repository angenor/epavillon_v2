//! **Un envoi qui n'avance plus le dit, et se relance.**
//!
//! Le défaut corrigé (31/10) : un serveur de courriel injoignable une minute
//! épuisait les essais du travail, qui mourait, et l'envoi restait « en cours »
//! pour toujours. Éprouvé par le vrai chemin : travail réservé sur sa file,
//! `fail_job()` du modèle, délais avancés à la main plutôt qu'attendus.

mod commun;

use commun::Bac;
use engagement::domain::mailing::{CreateMailingPayload, MailingPayload};
use engagement::jobs::send_mailing::SEND_MAILING;
use engagement::service::mailings;
use kernel::ErrorCode;
use std::collections::HashSet;
use uuid::Uuid;

fn quinze_adresses(edition: Uuid) -> CreateMailingPayload {
    CreateMailingPayload {
        event_id: edition,
        draft: MailingPayload {
            subject: Some("Consignes".into()),
            body_html: Some("<p>Bonjour,</p>".into()),
            audience: Some(vec![]),
            extra_addresses: Some((0..15).map(|i| format!("panne{i}@example.org")).collect()),
            cc: Some(vec![]),
            ..MailingPayload::default()
        },
    }
}

async fn envoi_parti(bac: &Bac, edition: Uuid, auteur: Uuid) -> Uuid {
    let envoi = mailings::creer(&bac.state, &bac.ctx(), auteur, &quinze_adresses(edition))
        .await
        .expect("brouillon");
    mailings::envoyer(&bac.state, &bac.ctx(), auteur, envoi.id)
        .await
        .expect("envoi");
    envoi.id
}

/// Le worker passe, puis les reprises sont avancées à maintenant, jusqu'à ce
/// qu'aucun travail d'envoi n'attende plus : les cinq essais de `fail_job()`
/// sans attendre leur minute.
async fn epuiser_les_essais(bac: &Bac) {
    for _ in 0..10 {
        let _ = commun::passer_le_worker(bac).await;
        let en_attente = sqlx::query!(
            "UPDATE platform.jobs SET run_at = now() WHERE task = $1 AND status = 'queued'",
            SEND_MAILING
        )
        .execute(bac.pool())
        .await
        .expect("reprises avancées")
        .rows_affected();
        if en_attente == 0 {
            return;
        }
    }
    panic!("les essais ne s'épuisent pas");
}

async fn travaux_morts(bac: &Bac) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = $1 AND status = 'dead'"#,
        SEND_MAILING
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage")
}

#[tokio::test]
async fn le_serveur_injoignable_passe_lenvoi_en_echec_partiellement_parti() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi_parti(&bac, t.edition, t.administratrice).await;

    bac.boite.tomber_apres(Some(4));
    epuiser_les_essais(&bac).await;

    assert_eq!(travaux_morts(&bac).await, 1);
    let envoi = mailings::detail(&bac.state, t.administratrice, id)
        .await
        .expect("détail");
    assert_eq!(envoi.status, "failed");
    assert!(envoi.failed_at.is_some());
    let motif = envoi.failure_reason.expect("motif");
    assert!(motif.contains("injoignable"), "{motif}");
    // Partiellement parti : quatre messages sont partis, onze attendent.
    assert_eq!(envoi.counts.sent, 4);
    assert_eq!(envoi.counts.queued, 11);
    assert_eq!(envoi.counts.sending, 0);

    let ligne = mailings::ecran(&bac.state, t.administratrice, t.edition)
        .await
        .expect("historique")
        .rows
        .into_iter()
        .find(|r| r.id == id)
        .expect("dans l'historique");
    assert_eq!(ligne.status, "failed");
    assert_eq!(ligne.counts.sent, 4);
    assert!(ligne.failure_reason.is_some());
}

#[tokio::test]
async fn la_relance_ne_sert_que_les_destinataires_en_attente() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi_parti(&bac, t.edition, t.administratrice).await;

    // Pas d'échec, pas de relance.
    let refus = mailings::relancer(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect_err("en cours");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);

    bac.boite.tomber_apres(Some(4));
    epuiser_les_essais(&bac).await;
    bac.boite.tomber_apres(None);

    let relance = mailings::relancer(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect("relance");
    assert_eq!(relance.status, "queued");
    assert_eq!(relance.relaunch_count, 1);
    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }

    let fini = mailings::detail(&bac.state, t.administratrice, id)
        .await
        .expect("détail");
    assert_eq!(fini.status, "sent");
    assert_eq!(fini.counts.sent, 15);
    // Le motif du dernier échec reste lisible après la relance.
    assert!(fini.failure_reason.is_some());

    let messages = bac.boite.messages();
    assert_eq!(messages.len(), 15, "aucun destinataire servi deux fois");
    let adresses: HashSet<_> = messages.iter().map(|m| m.to.clone()).collect();
    assert_eq!(adresses.len(), 15);

    // Une relance de trop est refusée.
    let refus = mailings::relancer(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect_err("déjà parti");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
}

/// Le travail mort avant que le worker sache poser l'échec (les envois bloqués
/// en production avant le 31/10) : le relevé le rattrape.
#[tokio::test]
async fn le_travail_mort_bascule_lenvoi_en_echec_au_releve() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let bloque = envoi_parti(&bac, t.edition, t.administratrice).await;
    sqlx::query!(
        "UPDATE platform.jobs
            SET status = 'dead', attempts = max_attempts,
                last_error = 'INTERNAL: erreur interne (serveur d''envoi injoignable, reprise plus tard : délai dépassé)'
          WHERE task = $1",
        SEND_MAILING
    )
    .execute(bac.pool())
    .await
    .expect("travail mort");
    // Un destinataire pris au moment de la mort : départ non confirmé.
    sqlx::query!(
        "UPDATE engagement.mailing_recipients SET status = 'sending'
          WHERE id = (SELECT min(id::text)::uuid FROM engagement.mailing_recipients
                       WHERE mailing_id = $1)",
        bloque
    )
    .execute(bac.pool())
    .await
    .expect("destinataire pris");

    // Un envoi qui progresse a son travail en file : il n'est pas touché.
    let vivant = envoi_parti(&bac, t.edition, t.administratrice).await;

    let ecran = mailings::ecran(&bac.state, t.administratrice, t.edition)
        .await
        .expect("historique");
    let statut = |id: Uuid| {
        ecran
            .rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.status.clone())
    };
    assert_eq!(statut(bloque).as_deref(), Some("failed"));
    assert_eq!(statut(vivant).as_deref(), Some("queued"));

    let envoi = mailings::detail(&bac.state, t.administratrice, bloque)
        .await
        .expect("détail");
    let motif = envoi.failure_reason.expect("motif");
    assert!(motif.contains("injoignable"), "{motif}");
    assert_eq!(envoi.counts.failed, 1);
    assert_eq!(envoi.counts.queued, 14);

    // Relancé, il part pour les quatorze en attente, pas pour celui dont le
    // départ n'est pas confirmé ; la clé de la relance ne heurte pas celle du
    // travail mort.
    mailings::relancer(&bac.state, &bac.ctx(), t.administratrice, bloque)
        .await
        .expect("relance");
    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }
    let fini = mailings::detail(&bac.state, t.administratrice, bloque)
        .await
        .expect("détail");
    assert_eq!(fini.status, "sent");
    assert_eq!(fini.counts.sent, 14);
    assert_eq!(fini.counts.failed, 1);

    // En échec, il se supprime ; arrêté aussi.
    let autre = envoi_parti(&bac, t.edition, t.administratrice).await;
    let pose = sqlx::query_scalar!(
        r#"SELECT engagement.fail_mailing($1, 'Le serveur de courriel est resté injoignable.') AS "pose!""#,
        autre
    )
    .fetch_one(bac.pool())
    .await
    .expect("échec posé");
    assert!(pose);
    mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, autre)
        .await
        .expect("suppression d'un envoi en échec");
}
