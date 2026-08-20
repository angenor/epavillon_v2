//! Traces, journalisation et propagation de `X-Request-Id`.
//!
//! Un identifiant de requête relie une page du navigateur, une trace Jaeger et
//! une ligne d'audit. Il n'a de valeur que s'il traverse les trois.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::TelemetryConfig;
use crate::error::{ApiError, Result};

/// Vidange les traces en attente à l'arrêt : sans cela, les dernières requêtes
/// d'un processus qui se termine n'atteignent jamais le collecteur.
pub struct TelemetryGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            let _ = provider.shutdown();
        }
    }
}

pub fn init(cfg: &TelemetryConfig) -> Result<TelemetryGuard> {
    let filtre = EnvFilter::try_new(&cfg.log_filter)
        .map_err(|e| ApiError::internal(format!("RUST_LOG illisible : {e}")))?;

    let journal = tracing_subscriber::fmt::layer().with_target(true);

    let Some(endpoint) = cfg.otlp_endpoint.as_deref() else {
        tracing_subscriber::registry()
            .with(filtre)
            .with(journal)
            .init();
        return Ok(TelemetryGuard { provider: None });
    };

    let exporteur = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/v1/traces", endpoint.trim_end_matches('/')))
        .build()
        .map_err(|e| ApiError::internal(format!("exporteur OTLP : {e}")))?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporteur)
        .with_resource(
            Resource::builder()
                .with_attributes([KeyValue::new("service.name", cfg.service_name.clone())])
                .build(),
        )
        .build();

    let couche_otel = tracing_opentelemetry::layer().with_tracer(provider.tracer("epavillon"));

    tracing_subscriber::registry()
        .with(filtre)
        .with(journal)
        .with(couche_otel)
        .init();

    Ok(TelemetryGuard {
        provider: Some(provider),
    })
}
