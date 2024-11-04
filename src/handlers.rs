use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::metrics::Metrics;

#[tracing::instrument]
pub async fn index(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 GET:index");
    let trace_id = find_current_trace_id();
    metrics.index_counter.add(1, &[]);
    (StatusCode::OK, format!("Hello {:?}!", trace_id))
}

#[tracing::instrument]
pub async fn health(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 GET:health");
    metrics.health_counter.add(1, &[]);
    (StatusCode::OK, "Healthy! 🏅".to_string())
}
