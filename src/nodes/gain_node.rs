use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Gain", category = "Processors")]
pub struct GainNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "0.0", min = -60.0, max = 20.0)]
    pub gain_db: f64,

    #[serde(skip)]
    gain_linear: f64,
}

impl Default for GainNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            gain_db: 0.0,
            gain_linear: 1.0,
        }
    }
}

#[async_trait]
impl ProcessingNode for GainNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(gain_db) = config.get("gain_db").and_then(|v| v.as_f64()) {
            self.gain_db = gain_db;
        }

        // Convert dB to linear
        self.gain_linear = 10_f64.powf(self.gain_db / 20.0);

        Ok(())
    }

    async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
        // Apply gain to all payload channels
        for (_key, data) in frame.payload.iter_mut() {
            let mut samples = data.as_ref().clone();
            for sample in samples.iter_mut() {
                *sample *= self.gain_linear;
            }
            *data = std::sync::Arc::new(samples);
        }

        Ok(frame)
    }
}
