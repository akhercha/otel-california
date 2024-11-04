use opentelemetry::{global, metrics::Counter};

#[derive(Debug)]
pub struct Metrics {
    pub index_counter: Counter<u64>,
    pub health_counter: Counter<u64>,
}

impl Metrics {
    pub fn new() -> Self {
        let meter = global::meter("metrics.opentelemtry");

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

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
