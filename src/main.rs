use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use metrics::Metrics;
use telemetry::init_telemetry;

pub mod handlers;
pub mod metrics;
pub mod telemetry;

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
    init_telemetry()?;
    let metrics = Arc::new(Metrics::new());

    let app = app(metrics.clone());
    let addr = &"0.0.0.0:3003".parse::<SocketAddr>()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
