//! Relais d'outbox et file de travaux — `cargo run -p worker`.

mod consumers;
mod jobs;
mod outbox;
mod registry;

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::events::ConsumerRegistry;
use time::OffsetDateTime;

use crate::consumers::telemetry::TelemetryConsumer;
use crate::registry::JobRegistry;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration refusée au démarrage : {e}");
        std::process::exit(1);
    });

    let _traces = kernel::telemetry::init(&config.telemetry).unwrap_or_else(|e| {
        eprintln!("Télémétrie : {e}");
        std::process::exit(1);
    });

    let db = Db::connect(config.database_url.expose(), 8)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("base injoignable : {e}");
            std::process::exit(1);
        });

    let consommateurs = ConsumerRegistry::new().register(TelemetryConsumer);
    let courrier = kernel::mail::build(&config.mail);
    let travaux =
        JobRegistry::new().register_all(identity::job_handlers(db.clone(), &config, courrier));

    // Les travaux récurrents se replanifient eux-mêmes ; le démarrage ne fait
    // que **réarmer** la chaîne, au cas où sa dernière occurrence serait morte
    // avant d'avoir posé la suivante. La clé d'unicité porte le jour : dix
    // redémarrages dans la journée n'en produisent pas dix purges.
    armer_les_recurrents(&db).await;

    tracing::info!(worker = %config.worker_id, "worker démarré");

    let mut relais = tokio::spawn(outbox::run(db.clone(), consommateurs));
    let mut file = tokio::spawn(jobs::run(db.clone(), config.worker_id.clone(), travaux));

    tokio::select! {
        _ = tokio::signal::ctrl_c() => tracing::info!("arrêt demandé"),
        r = &mut relais => tracing::error!(?r, "relais d'outbox arrêté"),
        r = &mut file => tracing::error!(?r, "file de travaux arrêtée"),
    }

    // Abandonner les deux tâches AVANT de fermer le pool : le relais tient une
    // connexion d'écoute pour la vie du processus, et `close()` attend que
    // toutes les connexions soient rendues — sans cet abandon, Ctrl-C ne rend
    // jamais la main.
    relais.abort();
    file.abort();
    db.close().await;
}

async fn armer_les_recurrents(db: &Db) {
    let resultat = async {
        let mut tx = db.write(&RequestContext::background("jobs")).await?;
        let pose = identity::jobs::purge::planifier(&mut tx, OffsetDateTime::now_utc()).await?;
        tx.commit().await?;
        Ok::<_, kernel::error::ApiError>(pose)
    }
    .await;

    match resultat {
        Ok(true) => tracing::info!("purge des jetons planifiée pour aujourd'hui"),
        Ok(false) => {}
        Err(e) => tracing::error!(erreur = %e, "planification des travaux récurrents impossible"),
    }
}
