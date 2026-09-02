//! **Les trois chaînes récurrentes de B6 s'arment, se replanifient, et le
//! démarrage du worker n'en produit pas de doublon.**
//!
//! # Pourquoi ce test vit ici et non dans les modules
//!
//! Le worker arme les six chaînes du dépôt dans **une seule transaction**, et
//! les trois nouvelles viennent de deux crates que rien ne relie — `media` et
//! `engagement` ne se connaissent pas, et ne doivent pas se connaître. Seul un
//! test qui voit les deux peut vérifier qu'ils cohabitent : c'est le rôle de
//! `crates/api/tests/`, exactement comme pour le montage des routes.
//!
//! # Ce qu'il attrape, et qu'aucun autre test n'attrape
//!
//! **La file.** Un travail récurrent posé dans la file par défaut alors que son
//! gestionnaire en déclare une autre s'empile sans erreur, sans trace, et sans
//! que rien ne l'exécute jamais — le défaut trouvé en phase 4. Ici, on lit la
//! file écrite en base et on la compare à celle que le gestionnaire déclare.
//!
//! # L'heure vient de la BASE
//!
//! L'horloge du conteneur et celle du processus diffèrent de quelques secondes.
//! Un créneau calculé depuis Rust reste juste — la grille est ancrée à l'époque
//! Unix, pas à `now()` —, mais tout ce qui se compare à `now()` se lit en SQL.

use kernel::db::Db;
use kernel::testing::TestDb;
use std::time::Duration;
use time::OffsetDateTime;

/// Les trois chaînes de B6, telles que `worker::armer_les_recurrents` les pose.
/// Rend, pour chacune, si le créneau a été posé — faux quand il l'était déjà.
async fn armer(db: &Db, moment: OffsetDateTime) -> (bool, bool, bool) {
    let mut tx = db
        .write(&kernel::context::RequestContext::background("test"))
        .await
        .expect("transaction");

    let purge = media::jobs::purge::planifier(
        &mut tx,
        media::jobs::purge::prochaine_occurrence(moment, Duration::from_secs(6 * 3600)),
    )
    .await
    .expect("purge");
    let reconcile = media::jobs::reconcile::planifier(
        &mut tx,
        media::jobs::reconcile::prochaine_occurrence(moment, Duration::from_secs(24 * 3600)),
    )
    .await
    .expect("réconciliation");
    let partitions = engagement::jobs::partitions::planifier(
        &mut tx,
        engagement::jobs::partitions::prochaine_occurrence(moment, Duration::from_secs(24 * 3600)),
    )
    .await
    .expect("partitions");

    tx.commit().await.expect("validation");
    (purge, reconcile, partitions)
}

/// **Dix redémarrages dans la journée ne produisent pas dix purges.**
///
/// La clé d'unicité porte le créneau visé : deux démarrages du même créneau
/// visent le même instant, et `ux_jobs_idempotency` les confond.
#[tokio::test]
async fn le_demarrage_du_worker_rearme_la_chaine_sans_doublon() {
    let base = TestDb::new().await;
    let db = base.db();

    // **Ancré au DÉBUT d'un créneau, jamais sur `now()`.** La grille des
    // occurrences est ancrée à l'époque Unix : neuf minutes ajoutées à un
    // instant proche d'une frontière tombent dans le créneau SUIVANT, et
    // l'armement rend alors « posé » là où le test attend « déjà posé ». Parti
    // de l'heure courante, il échouait pendant les neuf minutes précédant
    // chaque multiple de six heures — trente-six minutes par jour, quatre
    // fenêtres, et un échec qu'on impute d'abord à ce qu'on vient d'écrire.
    const CRENEAU: i64 = 6 * 3600;
    let maintenant = OffsetDateTime::from_unix_timestamp(
        OffsetDateTime::now_utc()
            .unix_timestamp()
            .div_euclid(CRENEAU)
            * CRENEAU,
    )
    .expect("début du créneau courant");

    assert_eq!(
        armer(&db, maintenant).await,
        (true, true, true),
        "premier démarrage : les trois chaînes sont posées"
    );

    // Neuf redémarrages de plus, à des instants différents du même créneau.
    for minute in 1..10 {
        assert_eq!(
            armer(&db, maintenant + time::Duration::minutes(minute)).await,
            (false, false, false),
            "le créneau était déjà posé"
        );
    }

    for tache in [
        "media.purge_assets",
        "media.reconcile_quotas",
        "engagement.ensure_partitions",
    ] {
        let compte = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = $1"#,
            tache
        )
        .fetch_one(base.pool())
        .await
        .expect("comptage");
        assert_eq!(compte, 1, "{tache} : une occurrence, et une seule");
    }
}

/// **Chaque travail est posé dans la file que son gestionnaire écoute.**
///
/// C'est l'assertion la plus discrète du fichier, et celle qui coûterait le plus
/// cher : `NewJob::new()` pose la file par défaut, et un gestionnaire qui en
/// déclare une autre ne verrait jamais son propre travail.
#[tokio::test]
async fn chaque_travail_recurrent_atterrit_dans_la_file_de_son_gestionnaire() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let db = base.db();

    armer(&db, OffsetDateTime::now_utc()).await;

    let mut gestionnaires = media::job_handlers(db.clone(), &config);
    gestionnaires.extend(engagement::job_handlers(
        db.clone(),
        &config,
        engagement::GardedMailer::envelopper(&config.mail, db.clone()).expect("expéditeur"),
    ));

    for tache in [
        "media.purge_assets",
        "media.reconcile_quotas",
        "engagement.ensure_partitions",
    ] {
        let file = sqlx::query_scalar!(
            r#"SELECT queue AS "queue!" FROM platform.jobs WHERE task = $1"#,
            tache
        )
        .fetch_one(base.pool())
        .await
        .expect("lecture de la file");

        let declaree = gestionnaires
            .iter()
            .find(|g| g.task() == tache)
            .unwrap_or_else(|| panic!("{tache} n'a aucun gestionnaire"))
            .queue();

        assert_eq!(
            file, declaree,
            "{tache} est posée dans « {file} » alors que son gestionnaire écoute « {declaree} » : \
             le travail s'empilerait sans erreur et sans trace"
        );
    }
}
