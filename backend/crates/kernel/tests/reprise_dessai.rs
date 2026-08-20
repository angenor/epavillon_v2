//! Un site injoignable fait échouer le travail, le replanifie, puis le met en
//! file morte.
//!
//! Le relais de courriel est une route du **site** : quand il ne répond pas,
//! l'envoi échoue. Rien de particulier ne doit arriver — c'est le but. Le
//! travail est replanifié avec un délai croissant, et au bout de ses essais il
//! meurt en gardant son erreur. La replanification n'est écrite nulle part dans
//! le code : `platform.fail_job()` la porte, et ce test le prouve plutôt que de
//! le supposer.

use kernel::jobs::{self, NewJob, DEFAULT_QUEUE};
use kernel::mail::{MailError, Mailer, OutgoingMail, RelayMailer, MAIL_RELAY_UNREACHABLE};
use kernel::testing::TestDb;
use kernel::{Db, RequestContext};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

const TACHE: &str = "identity.send_verification_email";
/// Port réservé et jamais servi : la connexion est refusée tout de suite,
/// plutôt qu'après le délai d'attente du client.
const SITE_INJOIGNABLE: &str = "http://127.0.0.1:1/api/internal/mail";

#[derive(Debug)]
struct Etat {
    status: String,
    attempts: i16,
    max_attempts: i16,
    run_at: OffsetDateTime,
    last_error: Option<String>,
    payload: String,
}

async fn etat(base: &TestDb, id: Uuid) -> Etat {
    let l = sqlx::query!(
        r#"SELECT status::text AS "status!", attempts, max_attempts, run_at, last_error,
                  payload::text AS "payload!"
             FROM platform.jobs WHERE id = $1"#,
        id
    )
    .fetch_one(base.pool())
    .await
    .expect("relecture du travail");

    Etat {
        status: l.status,
        attempts: l.attempts,
        max_attempts: l.max_attempts,
        run_at: l.run_at,
        last_error: l.last_error,
        payload: l.payload,
    }
}

/// Un tour de worker : réserver, tenter l'envoi, marquer l'échec.
async fn un_tour(db: &Db, mailer: &dyn Mailer) -> Option<Uuid> {
    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let travaux = jobs::claim(&mut tx, DEFAULT_QUEUE, "worker-de-test", 10)
        .await
        .expect("réservation");
    tx.commit().await.expect("validation");

    let travail = travaux.into_iter().next()?;

    let echec = mailer
        .send(&OutgoingMail {
            message_id: travail.id.to_string(),
            to: "awa.diallo@example.org".to_owned(),
            locale: "fr".to_owned(),
            subject: "Vérifiez votre adresse".to_owned(),
            text: "https://epavillon.local/verifier?jeton=…".to_owned(),
            html: None,
        })
        .await
        .expect_err("le site est injoignable");

    assert!(
        matches!(echec, MailError::Unreachable(_)),
        "injoignable, pas refusé : {echec}"
    );

    let mut tx = db
        .write(&travail.context())
        .await
        .expect("transaction d'échec");
    // Ce que le gestionnaire d'envoi écrit : le code stable, et rien de
    // l'adresse ni du corps de la réponse.
    jobs::fail(
        &mut tx,
        travail.id,
        &format!("{MAIL_RELAY_UNREACHABLE} : relais injoignable"),
    )
    .await
    .expect("marquage de l'échec");
    tx.commit().await.expect("validation");

    Some(travail.id)
}

#[tokio::test]
async fn un_site_injoignable_replanifie_puis_met_en_file_morte() {
    let base = TestDb::new().await;
    let db = base.db();
    let mailer = RelayMailer::new(SITE_INJOIGNABLE.to_owned(), "secret-de-test".to_owned());

    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let id = jobs::enqueue(
        &mut tx,
        NewJob::new(
            TACHE,
            json!({ "to": "awa.diallo@example.org", "token": "clair" }),
        )
        .idempotent("jeton-42"),
    )
    .await
    .expect("mise en file")
    .expect("travail posé");
    tx.commit().await.expect("validation");

    // Premier échec : le travail retourne en file, plus tard.
    assert_eq!(un_tour(&db, &mailer).await, Some(id));
    let apres = etat(&base, id).await;
    assert_eq!(apres.status, "queued", "replanifié, pas perdu");
    assert_eq!(apres.attempts, 1);
    assert!(
        apres.run_at > OffsetDateTime::now_utc(),
        "le délai croissant repousse le prochain essai"
    );
    assert!(apres
        .last_error
        .as_deref()
        .is_some_and(|e| e.starts_with(MAIL_RELAY_UNREACHABLE)));

    // Le tour suivant ne réserve RIEN : l'heure du prochain essai n'est pas
    // venue. C'est ce qui empêche une file en panne de tourner à vide.
    assert_eq!(
        un_tour(&db, &mailer).await,
        None,
        "un travail replanifié n'est pas réservable avant son heure"
    );

    // On avance l'horloge du travail plutôt que d'attendre : ce qu'on éprouve
    // est le passage en file morte, pas la patience du délai.
    let maximum = apres.max_attempts;
    for essai in 1..maximum {
        sqlx::query!("UPDATE platform.jobs SET run_at = now() WHERE id = $1", id)
            .execute(base.pool())
            .await
            .expect("avance de l'échéance");
        assert_eq!(un_tour(&db, &mailer).await, Some(id));
        assert_eq!(etat(&base, id).await.attempts, essai + 1);
    }

    let mort = etat(&base, id).await;
    assert_eq!(
        mort.status, "dead",
        "au-delà de ses essais, le travail meurt — il ne tourne pas indéfiniment"
    );
    assert_eq!(mort.attempts, maximum);

    // La charge utile porte un jeton en clair : elle part avec la mort du
    // travail, sinon un secret durable resterait dans une table qu'on relit.
    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    assert!(jobs::redact_dead(&mut tx, id)
        .await
        .expect("effacement de la charge utile"));
    tx.commit().await.expect("validation");
    assert_eq!(etat(&base, id).await.payload, "{}");
}

/// Un travail qu'un worker tué a laissé réservé revient à la file. La base sait
/// le VOIR — l'alerte `travaux_bloques` — mais ne le répare pas :
/// `claim_jobs()` ne prend que les `queued`.
#[tokio::test]
async fn un_travail_laisse_reserve_revient_a_la_file() {
    let base = TestDb::new().await;
    let db = base.db();

    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let id = jobs::enqueue(&mut tx, NewJob::new(TACHE, json!({})))
        .await
        .expect("mise en file")
        .expect("travail posé");
    jobs::claim(&mut tx, DEFAULT_QUEUE, "worker-tue", 10)
        .await
        .expect("réservation");
    tx.commit().await.expect("validation");

    assert_eq!(etat(&base, id).await.status, "running");

    // Le bail n'est pas expiré : rien ne bouge, et c'est essentiel — reprendre
    // trop tôt ferait tourner le travail DEUX FOIS, en parallèle du worker
    // vivant.
    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let reprises = jobs::reclaim_stalled(&mut tx, DEFAULT_QUEUE, 30.0 * 60.0)
        .await
        .expect("reprise");
    tx.commit().await.expect("validation");
    assert_eq!(reprises, 0);

    sqlx::query!(
        "UPDATE platform.jobs SET locked_at = now() - interval '2 hours' WHERE id = $1",
        id
    )
    .execute(base.pool())
    .await
    .expect("vieillissement du verrou");

    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let reprises = jobs::reclaim_stalled(&mut tx, DEFAULT_QUEUE, 30.0 * 60.0)
        .await
        .expect("reprise");
    tx.commit().await.expect("validation");

    assert_eq!(reprises, 1);
    let repris = etat(&base, id).await;
    assert_eq!(repris.status, "queued");
    assert_eq!(
        repris.attempts, 1,
        "l'essai déjà compté n'est pas rendu : un travail repris trois fois meurt avant ses cinq essais"
    );
}
