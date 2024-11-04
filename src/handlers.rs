use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use serde_json::json;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::metrics::Metrics;

#[tracing::instrument]
pub async fn index(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 index");
    let trace_id = find_current_trace_id();
    dbg!(&trace_id);
    metrics.index_counter.add(1, &[]);
    axum::Json(json!({ "my_trace_id": trace_id }))
}

#[tracing::instrument]
pub async fn health(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 health");
    metrics.health_counter.add(1, &[]);
    axum::Json(json!({ "status" : "UP" }))
}
