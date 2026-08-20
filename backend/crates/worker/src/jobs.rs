//! Boucle de travaux différés.
//!
//! La replanification n'est pas réécrite : `platform.fail_job()` porte déjà le
//! délai croissant et la file morte (principe VIII).

use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, DEFAULT_QUEUE};
use std::time::Duration;

use crate::registry::JobRegistry;

const REPOS: Duration = Duration::from_secs(2);
const LOT: i32 = 10;

/// Bail d'exécution avant qu'un travail réservé soit rendu à la file.
///
/// Franchement plus long que les quinze minutes de l'alerte `travaux_bloques`
/// de `analytics.v_operational_health` : ce seuil-là sert à VOIR une panne,
/// celui-ci à reprendre un travail. Les confondre ferait tourner deux fois, en
/// parallèle du worker vivant, un rafraîchissement analytique un peu long.
const BAIL: f64 = 30.0 * 60.0;
const RYTHME_REPRISE: Duration = Duration::from_secs(60);

pub async fn run(db: Db, worker_id: String, registry: JobRegistry) -> Result<()> {
    let mut prochaine_reprise = tokio::time::Instant::now();

    loop {
        if tokio::time::Instant::now() >= prochaine_reprise {
            if let Err(e) = reprendre_les_bloques(&db).await {
                tracing::error!(erreur = %e, "reprise des travaux bloqués impossible");
            }
            prochaine_reprise = tokio::time::Instant::now() + RYTHME_REPRISE;
        }

        let travaux = match reserver(&db, &worker_id).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(erreur = %e, "réservation de travaux impossible");
                tokio::time::sleep(REPOS).await;
                continue;
            }
        };

        if travaux.is_empty() {
            tokio::time::sleep(REPOS).await;
            continue;
        }

        for travail in travaux {
            executer(&db, &registry, travail).await;
        }
    }
}

async fn reserver(db: &Db, worker_id: &str) -> Result<Vec<ClaimedJob>> {
    let mut tx = db.write(&RequestContext::background("jobs")).await?;
    let travaux = jobs::claim(&mut tx, DEFAULT_QUEUE, worker_id, LOT).await?;
    tx.commit().await?;
    Ok(travaux)
}

async fn reprendre_les_bloques(db: &Db) -> Result<()> {
    let mut tx = db.write(&RequestContext::background("jobs")).await?;
    let reprises = jobs::reclaim_stalled(&mut tx, DEFAULT_QUEUE, BAIL).await?;
    tx.commit().await?;

    if reprises > 0 {
        tracing::warn!(
            reprises,
            "travaux rendus à la file après expiration du verrou"
        );
    }
    Ok(())
}

async fn executer(db: &Db, registry: &JobRegistry, travail: ClaimedJob) {
    let span = tracing::info_span!("job", tache = %travail.task, travail = %travail.id);
    let _garde = span.enter();

    let Some(handler) = registry.get(&travail.task) else {
        let message = format!("aucun gestionnaire pour la tâche « {} »", travail.task);
        tracing::error!("{message}");
        marquer(db, &travail, Err(message)).await;
        return;
    };

    match handler.run(&travail).await {
        Ok(()) => marquer(db, &travail, Ok(())).await,
        Err(e) => {
            tracing::warn!(erreur = %e, "travail en échec");
            marquer(db, &travail, Err(e.to_string())).await;
            if handler.carries_secret() {
                effacer_si_mort(db, &travail).await;
            }
        }
    }
}

/// `succeed()` vide la charge utile d'un travail réussi ; `platform.fail_job()`
/// ne vide jamais celle d'un travail mort — et c'est justifié pour les tâches
/// dont la charge utile est la seule matière de diagnostic. Pour celles qui
/// transportent un jeton en clair, ce serait un secret durable dans une table
/// qu'on relit : elles se déclarent, et leur charge utile part avec elles.
async fn effacer_si_mort(db: &Db, travail: &ClaimedJob) {
    let resultat = async {
        let mut tx = db.write(&travail.context()).await?;
        let efface = jobs::redact_dead(&mut tx, travail.id).await?;
        tx.commit().await?;
        Ok::<_, kernel::error::ApiError>(efface)
    }
    .await;

    match resultat {
        Ok(true) => tracing::info!("travail mort : charge utile effacée, elle portait un secret"),
        Ok(false) => {}
        Err(e) => tracing::error!(erreur = %e, "effacement de la charge utile impossible"),
    }
}

/// Une panne passagère de la base au moment de clore ne doit pas tuer la boucle
/// — et avec elle le relais d'outbox, qui partage le processus. Le travail
/// reste réservé ; `reprendre_les_bloques` le rendra à la file.
///
/// À savoir, et qui vaut aussi après un redémarrage : un travail dont la
/// RÉUSSITE n'a pas pu être écrite sera réexécuté. La file est « au moins une
/// fois », jamais « exactement une fois ».
async fn marquer(db: &Db, travail: &ClaimedJob, issue: std::result::Result<(), String>) {
    let ctx = travail.context();
    let resultat = async {
        let mut tx = db.write(&ctx).await?;
        match &issue {
            Ok(()) => jobs::succeed(&mut tx, travail.id).await?,
            Err(message) => jobs::fail(&mut tx, travail.id, message).await?,
        }
        tx.commit().await
    }
    .await;

    if let Err(e) = resultat {
        tracing::error!(erreur = %e, "marquage du travail impossible, il sera repris");
    }
}
