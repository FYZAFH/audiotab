use super::DataFrame;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Context passed to nodes during processing
#[derive(Clone, Debug)]
pub struct NodeContext {
    pub node_id: String,
    pub config: Value,
}

/// Base trait that all processing nodes must implement
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Initialize the node with configuration
    async fn on_create(&mut self, config: Value) -> Result<()> {
        let _ = config;
        Ok(())
    }

    /// Process a single data frame
    async fn process(&mut self, input: DataFrame) -> Result<DataFrame>;

    /// Cleanup when node is destroyed
    async fn on_destroy(&mut self) -> Result<()> {
        Ok(())
    }
}
