use crate::core::{ProcessingNode, DataFrame};
use crate::observability::NodeMetrics;
use super::ErrorPolicy;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

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

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(frame) = rx.recv().await {
            let start = self.metrics.start_processing();

            // Try to process the frame using the inner node's process() method
            // This allows us to catch errors on a per-frame basis
            let result = self.inner.process(frame.clone()).await;

            match result {
                Ok(output) => {
                    // Success - forward output
                    self.metrics.finish_processing(start);
                    self.metrics.record_frame_processed();

                    if tx.send(output).await.is_err() {
                        break;
                    }
                }
                Err(_) => {
                    // Error occurred
                    self.metrics.record_error();

                    match &self.error_policy {
                        ErrorPolicy::Propagate => {
                            return Err(anyhow::anyhow!("Node error"));
                        }
                        ErrorPolicy::SkipFrame => {
                            // Just skip this frame
                            continue;
                        }
                        ErrorPolicy::UseDefault(default_frame) => {
                            if tx.send(default_frame.clone()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
