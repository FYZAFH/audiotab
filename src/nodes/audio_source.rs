use crate::core::{ProcessingNode, DataFrame};
use crate::visualization::RingBufferWriter;
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Audio Source", category = "Sources")]
pub struct AudioSourceNode {
    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u32,

    #[param(default = "1024", min = 64.0, max = 8192.0)]
    pub buffer_size: u32,

    #[serde(skip)]
    sequence: u64,

    #[serde(skip)]
    ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,
}

impl Default for AudioSourceNode {
    fn default() -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            buffer_size: 1024,
            sequence: 0,
            ring_buffer: None,
        }
    }
}

#[async_trait]
impl ProcessingNode for AudioSourceNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(sr) = config.get("sample_rate").and_then(|v| v.as_u64()) {
            self.sample_rate = sr as u32;
        }
        if let Some(bs) = config.get("buffer_size").and_then(|v| v.as_u64()) {
            self.buffer_size = bs as u32;
        }
        Ok(())
    }

    async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
        // Generate silent audio for now (will be replaced with real capture)
        let samples = vec![0.0; self.buffer_size as usize];

        // Write to ring buffer
        if let Some(rb) = &self.ring_buffer {
            if let Ok(mut writer) = rb.lock() {
                let _ = writer.write(&vec![samples.clone()]); // Single channel for now
            }
        }

        frame.payload.insert(
            "main_channel".to_string(),
            std::sync::Arc::new(samples),
        );

        self.sequence += 1;
        frame.sequence_id = self.sequence;

        Ok(frame)
    }
}
