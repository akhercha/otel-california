use axum::{extract::State, response::IntoResponse, routing::get, Router};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use opentelemetry::{
    global,
    metrics::{Counter, Meter},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{reader::DefaultTemporalitySelector, MeterProviderBuilder, PeriodicReader},
    Resource,
};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Debug)]
struct Metrics {
    index_counter: Counter<u64>,
    health_counter: Counter<u64>,
}

impl Metrics {
    fn new(meter: &Meter) -> Self {
        let index_counter = meter
            .u64_counter("index_requests_total")
            .with_description("Total number of index requests")
            .init();

        let health_counter = meter
            .u64_counter("health_requests_total")
            .with_description("Total number of health requests")
            .init();

        Metrics {
            index_counter,
            health_counter,
        }
    }
}

fn init_meter() -> anyhow::Result<Arc<Metrics>> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317")
        .build_metrics_exporter(Box::new(DefaultTemporalitySelector::new()))?;

    let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(5))
        .with_timeout(std::time::Duration::from_secs(5))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "otel-california",
        )]))
        .with_reader(reader)
        .build();

    // Set the global meter provider
    global::set_meter_provider(meter_provider);

    // Create a Meter
    let meter = global::meter("otel-california");

    // Create Metrics struct
    let metrics = Metrics::new(&meter);

    Ok(Arc::new(metrics))
}

#[tracing::instrument]
async fn index(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 index");
    let trace_id = find_current_trace_id();
    dbg!(&trace_id);
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Increment the index counter
    metrics.index_counter.add(1, &[]);

    axum::Json(json!({ "my_trace_id": trace_id }))
}

#[tracing::instrument]
async fn health(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 health");

    // Increment the health counter
    metrics.health_counter.add(1, &[]);

    axum::Json(json!({ "status" : "UP" }))
}

fn app(metrics: Arc<Metrics>) -> Router {
    let metrics_layer = HttpMetricsLayerBuilder::new()
        .with_service_name("otel-california".to_string())
        .with_labels(
            vec![("INDEX".to_string(), "HEALTH".to_string())]
                .into_iter()
                .collect(),
        )
        .build();

    Router::new()
        .merge(metrics_layer.routes())
        .route("/health", get(health))
        .route("/", get(index))
        .layer(OtelInResponseLayer::default())
        .layer(OtelAxumLayer::default())
        .layer(metrics_layer)
        .with_state(metrics)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers()?;
    let metrics = init_meter()?;

    let app = app(metrics.clone());
    let addr = &"0.0.0.0:3003".parse::<SocketAddr>()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
