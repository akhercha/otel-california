use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::{BatchConfig, LoggerProvider};
use opentelemetry_sdk::metrics::reader::DefaultTemporalitySelector;
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader};
use opentelemetry_sdk::{runtime, trace::BatchConfigBuilder};
use opentelemetry_sdk::{
    trace::{Config, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing::level_filters::LevelFilter;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_telemetry(collection_endpoint: String) -> anyhow::Result<()> {
    let tracing_subscriber = tracing_subscriber::registry()
        .with(build_otel_layer()?)
        .with(LevelFilter::from_level(Level::INFO))
        .with(tracing_subscriber::fmt::layer().pretty());

    let tracer_provider = init_tracer_provider(&collection_endpoint)?;
    let logger_provider = init_logs_provider(&collection_endpoint)?;
    init_meter_provider(&collection_endpoint)?;

    tracing_subscriber
        .with(OpenTelemetryLayer::new(tracer_provider))
        .with(OpenTelemetryTracingBridge::new(&logger_provider))
        .init();

    Ok(())
}

fn init_tracer_provider(collection_endpoint: &str) -> anyhow::Result<Tracer> {
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_batch_config(BatchConfigBuilder::default().build())
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                SERVICE_NAME,
                "otel-california-trace-service",
            )])),
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(collection_endpoint),
        )
        .install_batch(runtime::Tokio)
        .expect("Failed to install tracer provider");

    global::set_tracer_provider(provider.clone());
    Ok(provider.tracer("otel-california-subscriber"))
}

fn init_logs_provider(collection_endpoint: &str) -> anyhow::Result<LoggerProvider> {
    let logger = opentelemetry_otlp::new_pipeline()
        .logging()
        .with_batch_config(BatchConfig::default())
        .with_resource(Resource::new(vec![KeyValue::new(
            SERVICE_NAME,
            "otel-california-logs-service",
        )]))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(collection_endpoint),
        )
        .install_batch(runtime::Tokio)?;

    Ok(logger)
}

pub fn init_meter_provider(collection_endpoint: &str) -> anyhow::Result<()> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(collection_endpoint)
        .build_metrics_exporter(Box::new(DefaultTemporalitySelector::new()))?;

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(5))
        .build();

    let metrics_provider = MeterProviderBuilder::default()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            SERVICE_NAME,
            "otel-california-meter-service",
        )]))
        .build();

    // Set the global meter provider
    global::set_meter_provider(metrics_provider);

    Ok(())
}
