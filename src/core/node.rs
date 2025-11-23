use super::DataFrame;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

/// Base trait for all processing nodes in the pipeline
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Called once when node is instantiated with config from JSON
    async fn on_create(&mut self, config: Value) -> Result<()>;

    /// Legacy single-shot processing (Phase 1 compatibility)
    /// Will be deprecated in favor of run() for streaming pipelines
    async fn process(&self, _input: DataFrame) -> Result<DataFrame> {
        // Default implementation: not supported
        anyhow::bail!("Node does not support single-shot processing")
    }

    /// Async streaming processing loop
    /// Receives frames from rx channel, processes them, sends to tx channel
    /// Should run until rx channel is closed, then return
    async fn run(
        &self,
        _rx: mpsc::Receiver<DataFrame>,
        _tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        // Default implementation: not supported
        anyhow::bail!("Node does not support streaming processing")
    }
}
