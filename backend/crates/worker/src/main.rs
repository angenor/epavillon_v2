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
    let travaux = JobRegistry::new()
        .register_all(identity::job_handlers(
            db.clone(),
            &config,
            courrier.clone(),
        ))
        .register_all(org::job_handlers(db.clone(), &config, courrier))
        // Ce module n'envoie aucun courriel : son unique travail clôt les
        // appels échus, et le rappel d'échéance aux organisations appartient à
        // B6.
        .register_all(event::job_handlers(db.clone(), &config));

    // Les travaux récurrents se replanifient eux-mêmes ; le démarrage ne fait
    // que **réarmer** la chaîne, au cas où sa dernière occurrence serait morte
    // avant d'avoir posé la suivante. La clé d'unicité porte le jour : dix
    // redémarrages dans la journée n'en produisent pas dix purges.
    armer_les_recurrents(&db, &config).await;

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

async fn armer_les_recurrents(db: &Db, config: &Config) {
    let resultat = async {
        let mut tx = db.write(&RequestContext::background("jobs")).await?;
        let maintenant = OffsetDateTime::now_utc();
        let purge = identity::jobs::purge::planifier(&mut tx, maintenant).await?;
        // Le balayage des doublons se réarme de la même façon, et pour la même
        // raison : sa dernière tranche a pu mourir avant d'avoir posé la
        // suivante. La clé d'unicité porte le jour — dix redémarrages n'en
        // produisent pas dix.
        let balayage = org::jobs::duplicates::planifier(&mut tx, maintenant).await?;
        // Troisième chaîne, même patron. Sa grille est horaire plutôt que
        // journalière : la clé d'unicité porte le créneau visé, pas le jour.
        let cloture = event::jobs::autoclose::planifier(
            &mut tx,
            event::jobs::autoclose::prochaine_occurrence(
                maintenant,
                config.event.call_autoclose_interval,
            ),
        )
        .await?;
        tx.commit().await?;
        Ok::<_, kernel::error::ApiError>((purge, balayage, cloture))
    }
    .await;

    match resultat {
        Ok((purge, balayage, cloture)) => {
            if purge {
                tracing::info!("purge des jetons planifiée pour aujourd'hui");
            }
            if balayage {
                tracing::info!("balayage des doublons planifié pour aujourd'hui");
            }
            if cloture {
                tracing::info!("clôture des appels échus planifiée pour le prochain créneau");
            }
        }
        Err(e) => tracing::error!(erreur = %e, "planification des travaux récurrents impossible"),
    }
}
