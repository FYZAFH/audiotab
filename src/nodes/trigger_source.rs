use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Trigger Source", category = "Sources")]
pub struct TriggerSourceNode {
    #[output(name = "Trigger Out", data_type = "trigger")]
    _output: (),

    #[param(default = "\"periodic\"")]
    pub mode: String,

    #[param(default = "100", min = 1.0, max = 10000.0)]
    pub interval_ms: u64,
}

impl Default for TriggerSourceNode {
    fn default() -> Self {
        Self {
            _output: (),
            mode: "periodic".to_string(),
            interval_ms: 100,
        }
    }
}

#[async_trait]
impl ProcessingNode for TriggerSourceNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        Ok(frame)
    }
}
