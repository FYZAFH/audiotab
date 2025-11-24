use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Debug Sink", category = "Sinks")]
pub struct DebugSinkNode {
    #[input(name = "Data In", data_type = "any")]
    _input: (),

    #[param(default = "\"info\"")]
    pub log_level: String,
}

impl Default for DebugSinkNode {
    fn default() -> Self {
        Self {
            _input: (),
            log_level: "info".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for DebugSinkNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        println!("[{}] Frame {} with {} channels",
                 self.log_level,
                 frame.sequence_id,
                 frame.payload.len());
        Ok(frame)
    }
}
