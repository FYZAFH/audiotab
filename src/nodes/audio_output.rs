use crate::core::{DataFrame, ProcessingNode};
use crate::hal::DeviceChannels;
use crate::hal::format_converter::frame_to_packet;
use crate::hal::types::SampleFormat;
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

/// AudioOutputNode bridges processing pipeline to hardware output
///
/// Responsibilities:
/// - Receives DataFrame from upstream nodes
/// - Converts DataFrame → PacketBuffer using format_converter
/// - Sends PacketBuffer to output device channels
#[derive(StreamNode, Serialize, Deserialize)]
#[node_meta(name = "Audio Output", category = "Sinks")]
pub struct AudioOutputNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u64,

    #[param(default = "1", min = 1.0, max = 32.0)]
    pub num_channels: usize,

    #[serde(skip)]
    format: SampleFormat,

    #[serde(skip)]
    device_channels: Option<DeviceChannels>,
}

impl std::fmt::Debug for AudioOutputNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioOutputNode")
            .field("sample_rate", &self.sample_rate)
            .field("num_channels", &self.num_channels)
            .field("format", &self.format)
            .finish()
    }
}

impl Clone for AudioOutputNode {
    fn clone(&self) -> Self {
        Self {
            _input: (),
            sample_rate: self.sample_rate,
            num_channels: self.num_channels,
            format: self.format,
            device_channels: None, // Don't clone channels
        }
    }
}

impl AudioOutputNode {
    /// Create a new AudioOutputNode with injected DeviceChannels
    ///
    /// # Arguments
    /// * `channels` - DeviceChannels for sending PacketBuffers to hardware
    /// * `format` - The sample format expected by the output device
    pub fn new(channels: DeviceChannels, format: SampleFormat) -> Self {
        Self {
            _input: (),
            sample_rate: 48000,
            num_channels: 1,
            format,
            device_channels: Some(channels),
        }
    }
}

impl Default for AudioOutputNode {
    fn default() -> Self {
        Self {
            _input: (),
            sample_rate: 48000,
            num_channels: 1,
            format: SampleFormat::F32,
            device_channels: None,
        }
    }
}

#[async_trait]
impl ProcessingNode for AudioOutputNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(sr) = config.get("sample_rate").and_then(|v| v.as_u64()) {
            self.sample_rate = sr;
        }
        if let Some(nc) = config.get("num_channels").and_then(|v| v.as_u64()) {
            self.num_channels = nc as usize;
        }
        if let Some(fmt) = config.get("format").and_then(|v| v.as_str()) {
            self.format = match fmt {
                "I16" => SampleFormat::I16,
                "I24" => SampleFormat::I24,
                "I32" => SampleFormat::I32,
                "F32" => SampleFormat::F32,
                "F64" => SampleFormat::F64,
                "U8" => SampleFormat::U8,
                _ => SampleFormat::F32, // Default fallback
            };
        }
        Ok(())
    }

    async fn process(&mut self, input: DataFrame) -> Result<DataFrame> {
        // If no device channels configured, just pass through
        if self.device_channels.is_none() {
            return Ok(input);
        }

        // Skip empty frames (no payload data)
        if input.payload.is_empty() {
            return Ok(input);
        }

        // Try to send the frame to the device
        if let Some(ref channels) = self.device_channels {
            // Convert DataFrame to PacketBuffer
            let packet = frame_to_packet(&input, self.format, self.sample_rate)
                .map_err(|e| anyhow::anyhow!(
                    "Failed to convert frame to packet (format: {:?}, sample_rate: {}): {}",
                    self.format, self.sample_rate, e
                ))?;

            // Send packet to device (non-blocking)
            // If device can't accept, we drop the frame (this prevents blocking the pipeline)
            let _ = channels.empty_tx.try_send(packet);
        }

        // Pass through the input frame (AudioOutputNode doesn't modify data)
        Ok(input)
    }

    async fn on_destroy(&mut self) -> Result<()> {
        // Clean up resources if needed
        self.device_channels = None;
        Ok(())
    }
}
