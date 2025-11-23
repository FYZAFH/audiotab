use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct NodeMetrics {
    node_id: String,
    frames_processed: AtomicU64,
    errors_count: AtomicU64,
    total_latency_us: AtomicU64,
    latency_samples: AtomicU64,
}

impl NodeMetrics {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            frames_processed: AtomicU64::new(0),
            errors_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            latency_samples: AtomicU64::new(0),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn frames_processed(&self) -> u64 {
        self.frames_processed.load(Ordering::Relaxed)
    }

    pub fn errors_count(&self) -> u64 {
        self.errors_count.load(Ordering::Relaxed)
    }

    pub fn record_frame_processed(&self) {
        self.frames_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn start_processing(&self) -> Instant {
        Instant::now()
    }

    pub fn finish_processing(&self, start: Instant) {
        let latency_us = start.elapsed().as_micros() as u64;
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn avg_latency_us(&self) -> u64 {
        let samples = self.latency_samples.load(Ordering::Relaxed);
        if samples == 0 {
            return 0;
        }
        self.total_latency_us.load(Ordering::Relaxed) / samples
    }
}
