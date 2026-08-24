//! **Le journal d'expédition garde ses partitions d'avance** (écart n° 137).
//!
//! Le modèle amorce trois mois et annonce, en commentaire, un worker de
//! maintenance qui n'existait pas. Passé ce trimestre, tout tombe dans la
//! partition « fourre-tout » : **aucune écriture n'échoue** — c'est bien pour
//! cela qu'elle existe —, et c'est précisément ce qui rend le défaut invisible.
//! Seule la purge par bascule de partition, la seule raison du partitionnement,
//! cesse silencieusement de fonctionner.
//!
//! Ce test regarde donc `pg_class`, et non le comportement d'une insertion :
//! l'insertion réussirait dans les deux cas.

mod commun;

use commun::Bac;
use time::OffsetDateTime;

/// Le nom qu'une partition mensuelle porte, tel que
/// `platform.ensure_month_partition()` le compose.
async fn partition_existe(bac: &Bac, mois_a_venir: i32) -> bool {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM pg_class c
               JOIN pg_namespace n ON n.oid = c.relnamespace
               WHERE n.nspname = 'engagement'
                 AND c.relname = format('email_messages_%s',
                     to_char(date_trunc('month', now()) + make_interval(months => $1::int),
                             'YYYYMM'))
           ) AS "existe!""#,
        mois_a_venir
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de pg_class")
}

/// L'instant **de la base**, et non celui du processus : les deux horloges
/// diffèrent, et un créneau posé depuis Rust peut tomber dans le futur de la
/// base — `claim_jobs()` ne réserverait alors rien.
async fn maintenant(bac: &Bac) -> OffsetDateTime {
    sqlx::query_scalar!(r#"SELECT now() AS "maintenant!""#)
        .fetch_one(bac.pool())
        .await
        .expect("instant de la base")
}

/// Pose une occurrence à l'instant donné.
///
/// **L'INSTANT EST UN PARAMÈTRE, ET C'EST INDISPENSABLE.** La clé d'unicité du
/// travail porte l'horodatage **à la seconde** : deux appels qui relisent chacun
/// `now()` tombent, une fois sur dix, de part et d'autre d'une seconde, posent
/// deux clés différentes et font échouer un test qui mesure exactement le
/// contraire. Le second armement doit rejouer le MÊME créneau, comme le fait un
/// worker qu'on redémarre.
async fn armer(bac: &Bac, moment: OffsetDateTime) -> bool {
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    let pose = engagement::jobs::partitions::planifier(&mut tx, moment)
        .await
        .expect("planification");
    tx.commit().await.expect("validation");
    pose
}

/// **Le semis s'arrête à trois mois ; le travail va plus loin.**
#[tokio::test]
async fn le_travail_prepare_les_mois_que_le_semis_na_pas_amorces() {
    let bac = Bac::monter().await;

    assert!(
        partition_existe(&bac, 0).await,
        "le mois courant est amorcé"
    );
    assert!(partition_existe(&bac, 2).await, "le semis va jusqu'au +2");
    assert!(
        !partition_existe(&bac, 3).await,
        "le semis s'arrête là — c'est l'écart n° 137"
    );

    let moment = maintenant(&bac).await;
    assert!(armer(&bac, moment).await);
    let issues = commun::passer_le_worker(&bac).await;
    assert!(issues.iter().all(|i| i.is_ok()), "{issues:?}");

    for mois in 0..=3 {
        assert!(
            partition_existe(&bac, mois).await,
            "la partition du mois +{mois} manque"
        );
    }
}

/// **Le travail se replanifie, et un second armement du même créneau n'en
/// produit pas un second.**
#[tokio::test]
async fn le_travail_se_replanifie_sans_doublon() {
    let bac = Bac::monter().await;

    let moment = maintenant(&bac).await;
    assert!(armer(&bac, moment).await, "le créneau n'était pas posé");
    assert!(
        !armer(&bac, moment).await,
        "le démarrage du worker repose le même créneau : la clé d'unicité le confond"
    );
    assert_eq!(
        commun::compter_travaux(&bac, "engagement.ensure_partitions").await,
        1
    );

    let issues = commun::passer_le_worker(&bac).await;
    assert_eq!(issues.len(), 1);
    assert!(issues[0].is_ok(), "{:?}", issues[0]);

    assert_eq!(
        commun::compter_travaux(&bac, "engagement.ensure_partitions").await,
        2,
        "l'occurrence exécutée et la suivante"
    );

    let en_attente = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'engagement.ensure_partitions'
              AND status = 'queued' AND run_at > now()"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(en_attente, 1, "la chaîne continue");
}

/// **Rejouer le travail ne casse rien** : la fonction du modèle est idempotente,
/// et la file est « au moins une fois ».
#[tokio::test]
async fn rejouer_le_travail_est_sans_effet() {
    let bac = Bac::monter().await;

    let moment = maintenant(&bac).await;
    armer(&bac, moment).await;
    commun::passer_le_worker(&bac).await;

    let rejoues = commun::rejouer_les_travaux(&bac, "engagement.ensure_partitions").await;
    assert!(rejoues >= 1);
    let issues = commun::passer_le_worker(&bac).await;
    assert!(
        issues.iter().all(|i| i.is_ok()),
        "un second passage doit aboutir : {issues:?}"
    );

    for mois in 0..=3 {
        assert!(partition_existe(&bac, mois).await);
    }
}
