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

    tracing::info!(adresse = %adresse, "API en écoute");

    HttpServer::new(move || api::build_app(&etat))
        .bind(&adresse)?
        .run()
        .await
}
