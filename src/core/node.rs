use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use super::DataFrame;

/// Base trait for all processing nodes in the pipeline
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Called once when node is instantiated with config from JSON
    async fn on_create(&mut self, config: Value) -> Result<()>;

    /// Process a single DataFrame and return result
    /// Can handle both data flow and control flow signals
    async fn process(&self, input: DataFrame) -> Result<DataFrame>;
}
