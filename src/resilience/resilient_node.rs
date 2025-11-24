use crate::core::{ProcessingNode, DataFrame};
use crate::observability::NodeMetrics;
use super::ErrorPolicy;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct ResilientNode {
    inner: Box<dyn ProcessingNode>,
    metrics: Arc<NodeMetrics>,
    error_policy: ErrorPolicy,
}

impl ResilientNode {
    pub fn new(
        inner: Box<dyn ProcessingNode>,
        metrics: Arc<NodeMetrics>,
        error_policy: ErrorPolicy,
    ) -> Self {
        Self {
            inner,
            metrics,
            error_policy,
        }
    }
}

#[async_trait]
impl ProcessingNode for ResilientNode {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        self.inner.on_create(config).await
    }

    async fn process(&mut self, input: DataFrame) -> Result<DataFrame> {
        let start = self.metrics.start_processing();

        // Try to process the frame using the inner node's process() method
        let result = self.inner.process(input.clone()).await;

        match result {
            Ok(output) => {
                // Success - forward output
                self.metrics.finish_processing(start);
                self.metrics.record_frame_processed();
                Ok(output)
            }
            Err(e) => {
                // Error occurred
                self.metrics.record_error();

                match &self.error_policy {
                    ErrorPolicy::Propagate => {
                        Err(e)
                    }
                    ErrorPolicy::SkipFrame => {
                        // Return input unchanged
                        Ok(input)
                    }
                    ErrorPolicy::UseDefault(default_frame) => {
                        Ok(default_frame.clone())
                    }
                }
            }
        }
    }

    async fn on_destroy(&mut self) -> Result<()> {
        self.inner.on_destroy().await
    }
}
