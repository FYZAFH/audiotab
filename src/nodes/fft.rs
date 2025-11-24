use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "FFT", category = "Processors")]
pub struct FFTNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "FFT Out", data_type = "fft_result")]
    _output: (),

    #[param(default = "\"hann\"")]
    pub window_type: String,
}

impl Default for FFTNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            window_type: "hann".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for FFTNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        // Real FFT implementation will come in next phase
        Ok(frame)
    }
}
