//! Deux demandes du même travail avec la même clé n'en exécutent qu'une.
//!
//! La garantie n'est pas dans le code : `ux_jobs_idempotency` porte le couple
//! `(tâche, clé)` et l'INSERT ne fait rien en cas de conflit. C'est ce qui
//! empêche structurellement le double envoi — deux clics sur « renvoyer le
//! lien », deux requêtes concurrentes, ou la reprise d'un lot après panne.

use kernel::jobs::{self, NewJob, DEFAULT_QUEUE};
use kernel::testing::TestDb;
use kernel::RequestContext;
use serde_json::json;

const TACHE: &str = "identity.send_verification_email";

async fn mettre_en_file(db: &kernel::Db, cle: &str) -> Option<uuid::Uuid> {
    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let id = jobs::enqueue(
        &mut tx,
        NewJob::new(TACHE, json!({ "to": "awa.diallo@example.org" })).idempotent(cle),
    )
    .await
    .expect("mise en file");
    tx.commit().await.expect("validation");
    id
}

#[tokio::test]
async fn deux_demandes_de_meme_cle_ne_produisent_quun_travail() {
    let base = TestDb::new().await;
    let db = base.db();

    let premier = mettre_en_file(&db, "jeton-42").await;
    let second = mettre_en_file(&db, "jeton-42").await;

    assert!(premier.is_some(), "le premier est posé");
    assert!(
        second.is_none(),
        "le second ne l'est pas, et le dit — sans erreur"
    );

    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = $1"#,
        TACHE
    )
    .fetch_one(base.pool())
    .await
    .expect("comptage");
    assert_eq!(travaux, 1);
}

/// La clé n'est unique que **pour une tâche donnée** : deux tâches différentes
/// peuvent porter la même. Sans cela, l'identifiant d'un jeton bloquerait à la
/// fois son courriel de vérification et tout autre travail qui s'y référerait.
#[tokio::test]
async fn la_meme_cle_sur_deux_taches_donne_deux_travaux() {
    let base = TestDb::new().await;
    let db = base.db();

    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let a = jobs::enqueue(
        &mut tx,
        NewJob::new("identity.send_verification_email", json!({})).idempotent("jeton-42"),
    )
    .await
    .expect("première mise en file");
    let b = jobs::enqueue(
        &mut tx,
        NewJob::new("identity.send_password_reset_email", json!({})).idempotent("jeton-42"),
    )
    .await
    .expect("seconde mise en file");
    tx.commit().await.expect("validation");

    assert!(a.is_some() && b.is_some());
    assert_ne!(a, b);
}

/// Un travail **déjà réussi** garde sa clé : la redemander ne le rejoue pas.
/// C'est ce qui fait qu'un worker relancé après un envoi n'envoie pas deux fois
/// le même courriel — l'index couvre tous les états sauf `cancelled`.
#[tokio::test]
async fn un_travail_deja_reussi_ne_se_redemande_pas() {
    let base = TestDb::new().await;
    let db = base.db();

    let id = mettre_en_file(&db, "jeton-42").await.expect("travail posé");

    let mut tx = db
        .write(&RequestContext::background("test-jobs"))
        .await
        .expect("transaction");
    let reserves = jobs::claim(&mut tx, DEFAULT_QUEUE, "worker-de-test", 10)
        .await
        .expect("réservation");
    jobs::succeed(&mut tx, id).await.expect("réussite");
    tx.commit().await.expect("validation");

    assert_eq!(reserves.len(), 1);

    assert!(
        mettre_en_file(&db, "jeton-42").await.is_none(),
        "la clé vaut encore après la réussite"
    );

    // Et la charge utile est partie avec le succès : le travail garde sa trace,
    // pas son contenu (SC-009).
    let charge = sqlx::query_scalar!(
        r#"SELECT payload::text AS "payload!" FROM platform.jobs WHERE id = $1"#,
        id
    )
    .fetch_one(base.pool())
    .await
    .expect("relecture");
    assert_eq!(charge, "{}");
}
