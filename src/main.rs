pub mod handlers;
pub mod metrics;
pub mod telemetry;

use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use dotenv::dotenv;

use crate::metrics::Metrics;
use crate::telemetry::init_telemetry;

fn app(metrics: Arc<metrics::Metrics>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::index))
        .layer(OtelInResponseLayer::default())
        .layer(OtelAxumLayer::default())
        .with_state(metrics)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    init_telemetry(std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")?)?;
    let metrics = Arc::new(Metrics::new());

    let app = app(metrics.clone());
    let addr = &"0.0.0.0:3003".parse::<SocketAddr>()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
