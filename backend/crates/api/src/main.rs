//! Binaire HTTP — `cargo run -p api`. L'assemblage vit dans `lib.rs`.

use actix_web::HttpServer;
use api::state::AppState;
use kernel::config::Config;
use kernel::db::Db;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration refusée au démarrage : {e}");
        std::process::exit(1);
    });

    let _traces = kernel::telemetry::init(&config.telemetry).unwrap_or_else(|e| {
        eprintln!("Télémétrie : {e}");
        std::process::exit(1);
    });

    let db = Db::connect(config.database_url.expose(), 16)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("base injoignable : {e}");
            std::process::exit(1);
        });

    let adresse = config.api_bind_addr.clone();
    let etat = AppState::new(db, config).await.unwrap_or_else(|e| {
        tracing::error!("état de l'application refusé au démarrage : {e}");
        std::process::exit(1);
    });

    etat.realtime.ecouter(etat.db.pool());
    annoncer_l_arret(etat.realtime.clone());

    tracing::info!(adresse = %adresse, "API en écoute");

    HttpServer::new(move || api::build_app(&etat))
        .bind(&adresse)?
        // Sous les 10 s de `docker stop` : les flux se ferment d'eux-mêmes à
        // l'annonce d'arrêt, le reste a le temps de finir.
        .shutdown_timeout(8)
        .run()
        .await
}

/// SIGTERM ou Ctrl-C : les flux temps réel reçoivent `restart` et se ferment.
/// Sans cela, Actix attendrait leur fin, qui ne vient jamais. Actix écoute les
/// mêmes signaux de son côté et mène l'arrêt.
fn annoncer_l_arret(hub: std::sync::Arc<api::realtime::Hub>) {
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let Ok(mut terme) = signal(SignalKind::terminate()) else {
                return;
            };
            tokio::select! {
                _ = terme.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        hub.annoncer_arret();
    });
}
