pub mod handlers;
pub mod metrics;
pub mod middlewares;
pub mod telemetry;

use std::{net::SocketAddr, sync::Arc};

use axum::middleware;
use axum::{routing::get, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use dotenv::dotenv;
use middlewares::track_time;

use crate::metrics::Metrics;
use crate::telemetry::init_telemetry;

fn app(metrics: Arc<metrics::Metrics>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/", get(handlers::index))
        .layer(middleware::from_fn(track_time))
        .layer(OtelAxumLayer::default())
        .layer(OtelInResponseLayer)
        .with_state(metrics)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if let Ok(otel_exporter_endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        init_telemetry(otel_exporter_endpoint)?;
    }
    let metrics = Arc::new(Metrics::new());

    let app = app(metrics.clone());
    let addr = &"0.0.0.0:3003".parse::<SocketAddr>()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    use std::sync::mpsc;
    use std::{thread, time::Duration};

    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
    let (sender, receiver) = mpsc::channel();
    let _ = thread::spawn(move || {
        opentelemetry::global::shutdown_tracer_provider();
        sender.send(()).ok()
    });
    let shutdown_res = receiver.recv_timeout(Duration::from_millis(2_000));
    if shutdown_res.is_err() {
        tracing::error!("failed to shutdown OpenTelemetry");
    }
}
