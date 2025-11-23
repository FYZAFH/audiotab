use std::collections::HashMap;
use std::sync::Arc;
use super::NodeMetrics;

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub node_id: String,
    pub frames_processed: u64,
    pub errors_count: u64,
    pub avg_latency_us: u64,
}

pub struct MetricsCollector {
    metrics: HashMap<String, Arc<NodeMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    pub fn register(&mut self, node_id: impl Into<String>, metrics: Arc<NodeMetrics>) {
        self.metrics.insert(node_id.into(), metrics);
    }

    pub fn snapshot(&self) -> HashMap<String, MetricsSnapshot> {
        self.metrics
            .iter()
            .map(|(id, metrics)| {
                (
                    id.clone(),
                    MetricsSnapshot {
                        node_id: metrics.node_id().to_string(),
                        frames_processed: metrics.frames_processed(),
                        errors_count: metrics.errors_count(),
                        avg_latency_us: metrics.avg_latency_us(),
                    },
                )
            })
            .collect()
    }

    pub fn get_node_metrics(&self, node_id: &str) -> Option<Arc<NodeMetrics>> {
        self.metrics.get(node_id).cloned()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
        }
    }
}
