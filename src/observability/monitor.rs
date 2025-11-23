use super::MetricsCollector;

pub struct PipelineMonitor {
    collector: MetricsCollector,
}

impl PipelineMonitor {
    pub fn new(collector: MetricsCollector) -> Self {
        Self { collector }
    }

    pub fn generate_report(&self) -> String {
        let snapshot = self.collector.snapshot();

        if snapshot.is_empty() {
            return "No nodes registered".to_string();
        }

        let mut report = String::from("=== Pipeline Metrics ===\n");

        for (node_id, metrics) in snapshot.iter() {
            report.push_str(&format!(
                "\n[{}]\n  Frames: {} frames processed\n  Errors: {}\n  Avg Latency: {}μs\n",
                node_id,
                metrics.frames_processed,
                if metrics.errors_count > 0 {
                    format!("{} error{}", metrics.errors_count, if metrics.errors_count == 1 { "" } else { "s" })
                } else {
                    "0 errors".to_string()
                },
                metrics.avg_latency_us
            ));
        }

        report
    }

    pub fn collector(&self) -> &MetricsCollector {
        &self.collector
    }
}
