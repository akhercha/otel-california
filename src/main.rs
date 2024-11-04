use axum::{extract::State, response::IntoResponse, routing::get, Router};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{
    global,
    metrics::{Counter, Meter},
    KeyValue,
};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::{
    metrics::{reader::DefaultTemporalitySelector, MeterProviderBuilder, PeriodicReader},
    trace::{Config, Tracer},
    Resource,
};
use opentelemetry_sdk::{runtime, trace::BatchConfigBuilder};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

fn init_tracer_provider() -> anyhow::Result<Tracer> {
    let otel_endpoint = "http://localhost:4317".to_string();

    let batch_config = BatchConfigBuilder::default()
        .with_max_queue_size(2305843009213693951) // 🗿
        .build();

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otel_endpoint.to_string()),
        )
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "otel-california",
            )])),
        )
        .with_batch_config(batch_config)
        .install_batch(runtime::Tokio)
        .expect("Failed to install tracer provider");

    global::set_tracer_provider(provider.clone());
    Ok(provider.tracer(format!("{}{}", "otel-california", "_subscriber")))
}

fn init_logs_provider() -> anyhow::Result<LoggerProvider> {
    let otel_endpoint = "http://localhost:4317".to_string();
    let logger = opentelemetry_otlp::new_pipeline()
        .logging()
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "otel-california",
        )]))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otel_endpoint.to_string()),
        )
        .install_batch(runtime::Tokio)?;

    Ok(logger)
}

fn init_meter_provider() -> anyhow::Result<Meter> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317")
        .build_metrics_exporter(Box::new(DefaultTemporalitySelector::new()))?;

    let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(5))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "otel-california",
        )]))
        .build();

    // Set the global meter provider
    global::set_meter_provider(meter_provider);

    // Create a Meter
    let meter = global::meter("otel-california");
    Ok(meter)
}

#[tracing::instrument]
async fn index(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 index");
    let trace_id = find_current_trace_id();
    dbg!(&trace_id);
    metrics.index_counter.add(1, &[]);
    axum::Json(json!({ "my_trace_id": trace_id }))
}

#[tracing::instrument]
async fn health(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    tracing::info!("🌐 health");
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
    let tracing_subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::DEBUG,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(build_otel_layer()?);

    let tracer = init_tracer_provider()?;
    let logger_provider = init_logs_provider()?;
    let meter = init_meter_provider()?;

    let layer = OpenTelemetryTracingBridge::new(&logger_provider);
    tracing_subscriber
        .with(OpenTelemetryLayer::new(tracer))
        .with(layer)
        .init();

    let metrics = Arc::new(Metrics::new(&meter));

    let app = app(metrics.clone());
    let addr = &"0.0.0.0:3003".parse::<SocketAddr>()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
