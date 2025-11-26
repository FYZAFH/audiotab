use crate::core::{DataFrame, ProcessingNode};
use crate::hal::DeviceChannels;
use crate::hal::format_converter::packet_to_frame;
use crate::visualization::RingBufferWriter;
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// AudioInputNode bridges hardware device to processing pipeline
///
/// Responsibilities:
/// - Reads PacketBuffer from injected DeviceChannels
/// - Converts PacketBuffer → DataFrame using format_converter
/// - Returns buffers to device (ping-pong pattern)
/// - Writes to RingBufferWriter for visualization
#[derive(StreamNode, Serialize, Deserialize)]
#[node_meta(name = "Audio Input", category = "Sources")]
pub struct AudioInputNode {
    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u64,

    #[param(default = "1", min = 1.0, max = 32.0)]
    pub num_channels: usize,

    #[serde(skip)]
    format_str: String,

    #[serde(skip)]
    sequence: u64,

    #[serde(skip)]
    device_channels: Option<DeviceChannels>,

    #[serde(skip)]
    ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,
}

impl std::fmt::Debug for AudioInputNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioInputNode")
            .field("sample_rate", &self.sample_rate)
            .field("num_channels", &self.num_channels)
            .field("format", &self.format_str)
            .field("sequence", &self.sequence)
            .finish()
    }
}

impl Clone for AudioInputNode {
    fn clone(&self) -> Self {
        Self {
            _output: (),
            sample_rate: self.sample_rate,
            num_channels: self.num_channels,
            format_str: self.format_str.clone(),
            sequence: self.sequence,
            device_channels: None, // Don't clone channels
            ring_buffer: self.ring_buffer.clone(),
        }
    }
}

impl AudioInputNode {
    /// Create a new AudioInputNode with injected DeviceChannels
    ///
    /// # Arguments
    /// * `channels` - DeviceChannels for receiving PacketBuffers from hardware
    /// * `ring_buffer` - Optional RingBufferWriter for visualization
    pub fn new(
        channels: DeviceChannels,
        ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,
    ) -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            num_channels: 1,
            format_str: "F32".to_string(),
            sequence: 0,
            device_channels: Some(channels),
            ring_buffer,
        }
    }
}

impl Default for AudioInputNode {
    fn default() -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            num_channels: 1,
            format_str: "F32".to_string(),
            sequence: 0,
            device_channels: None,
            ring_buffer: None,
        }
    }
}

#[async_trait]
impl ProcessingNode for AudioInputNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(sr) = config.get("sample_rate").and_then(|v| v.as_u64()) {
            self.sample_rate = sr;
        }
        if let Some(nc) = config.get("num_channels").and_then(|v| v.as_u64()) {
            self.num_channels = nc as usize;
        }
        if let Some(fmt) = config.get("format").and_then(|v| v.as_str()) {
            self.format_str = fmt.to_string();
        }
        Ok(())
    }

    async fn process(&mut self, _input: DataFrame) -> Result<DataFrame> {
        // Try to receive a packet from the device
        if let Some(ref channels) = self.device_channels {
            // Use try_recv to avoid blocking (non-blocking receive)
            match channels.filled_rx.try_recv() {
                Ok(packet) => {
                    // Get packet format information for error context
                    let format_name = match &packet.data {
                        crate::hal::types::SampleData::I16(_) => "I16",
                        crate::hal::types::SampleData::I24(_) => "I24",
                        crate::hal::types::SampleData::I32(_) => "I32",
                        crate::hal::types::SampleData::F32(_) => "F32",
                        crate::hal::types::SampleData::F64(_) => "F64",
                        crate::hal::types::SampleData::U8(_) => "U8",
                        crate::hal::types::SampleData::Bytes(_) => "Bytes",
                    };
                    let num_channels = packet.num_channels;

                    // Increment sequence for this frame
                    self.sequence += 1;

                    // Convert PacketBuffer to DataFrame
                    let frame = packet_to_frame(&packet, self.sequence)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to convert packet to frame (format: {}, channels: {}): {}",
                            format_name, num_channels, e
                        ))?;

                    // Write to ring buffer for visualization if available
                    if let Some(ref rb) = self.ring_buffer {
                        if let Ok(mut writer) = rb.lock() {
                            // Extract channel data for ring buffer
                            let mut channels_data = Vec::new();
                            for ch in 0..self.num_channels {
                                if let Some(ch_data) = frame.payload.get(&format!("ch{}", ch)) {
                                    channels_data.push(ch_data.as_ref().clone());
                                }
                            }
                            if !channels_data.is_empty() {
                                if let Err(e) = writer.write(&channels_data) {
                                    eprintln!("Ring buffer write failed: {}", e);
                                }
                            }
                        }
                    }

                    // Return the buffer to the device (ping-pong pattern)
                    let _ = channels.empty_tx.send(packet);

                    Ok(frame)
                }
                Err(_) => {
                    // No packet available - return empty frame
                    // This is not an error, just means device hasn't produced new data yet
                    self.sequence += 1;  // Increment for consistency
                    Ok(DataFrame::new(0, self.sequence))
                }
            }
        } else {
            // No device channels configured - return empty frame
            self.sequence += 1;
            Ok(DataFrame::new(0, self.sequence))
        }
    }

    async fn on_destroy(&mut self) -> Result<()> {
        // Clean up resources if needed
        self.device_channels = None;
        self.ring_buffer = None;
        Ok(())
    }
}
