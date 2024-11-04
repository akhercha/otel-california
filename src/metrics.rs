use opentelemetry::{global, metrics::Counter, KeyValue};

#[derive(Debug)]
pub struct Metrics {
    pub index_counter: Counter<u64>,
    pub health_counter: Counter<u64>,
}

impl Metrics {
    pub fn new() -> Self {
        let common_scope_attributes = vec![KeyValue::new("crate", "metrics")];
        let meter = global::meter_with_version(
            "metrics.opentelemtry",
            Some("0.17"),
            Some("https://opentelemetry.io/schemas/1.2.0"),
            Some(common_scope_attributes.clone()),
        );

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
