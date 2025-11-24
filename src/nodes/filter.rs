use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Filter", category = "Processors")]
pub struct FilterNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "\"lowpass\"")]
    pub filter_type: String,

    #[param(default = "1000.0", min = 20.0, max = 20000.0)]
    pub cutoff_hz: f64,
}

impl Default for FilterNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            filter_type: "lowpass".to_string(),
            cutoff_hz: 1000.0,
        }
    }
}

#[async_trait]
impl ProcessingNode for FilterNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        Ok(frame)
    }
}
