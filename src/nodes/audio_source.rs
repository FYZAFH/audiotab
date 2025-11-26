use crate::core::{ProcessingNode, DataFrame};
use crate::hal::DeviceChannels;
use crate::hal::format_converter::packet_to_frame;
use crate::visualization::RingBufferWriter;
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(StreamNode, Serialize, Deserialize)]
#[node_meta(name = "Audio Source", category = "Sources")]
pub struct AudioSourceNode {
    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u32,

    #[param(default = "1024", min = 64.0, max = 8192.0)]
    pub buffer_size: u32,

    #[param(default = "1", min = 1.0, max = 32.0)]
    pub num_channels: usize,

    #[serde(skip)]
    sequence: u64,

    #[serde(skip)]
    pub ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,

    #[serde(skip)]
    device_channels: Option<DeviceChannels>,
}

// Manual Debug implementation since DeviceChannels doesn't implement Debug
impl std::fmt::Debug for AudioSourceNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSourceNode")
            .field("sample_rate", &self.sample_rate)
            .field("buffer_size", &self.buffer_size)
            .field("num_channels", &self.num_channels)
            .field("sequence", &self.sequence)
            .field("has_device", &self.device_channels.is_some())
            .finish()
    }
}

// Manual Clone implementation since DeviceChannels doesn't implement Clone correctly
impl Clone for AudioSourceNode {
    fn clone(&self) -> Self {
        Self {
            _output: (),
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
            num_channels: self.num_channels,
            sequence: self.sequence,
            ring_buffer: self.ring_buffer.clone(),
            device_channels: None, // Don't clone device channels
        }
    }
}

impl Default for AudioSourceNode {
    fn default() -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            buffer_size: 1024,
            num_channels: 1,
            sequence: 0,
            ring_buffer: None,
            device_channels: None,
        }
    }
}

impl AudioSourceNode {
    /// Create a new AudioSourceNode with injected DeviceChannels
    ///
    /// # Arguments
    /// * `channels` - DeviceChannels for receiving PacketBuffers from hardware device
    /// * `ring_buffer` - Optional RingBufferWriter for visualization
    ///
    /// This constructor enables real audio input from a device.
    /// If no device is available, the node falls back to silent audio.
    pub fn with_device(
        channels: DeviceChannels,
        ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,
    ) -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            buffer_size: 1024,
            num_channels: 1,
            sequence: 0,
            ring_buffer,
            device_channels: Some(channels),
        }
    }

    /// Set or update the ring buffer writer
    ///
    /// # Arguments
    /// * `ring_buffer` - Optional RingBufferWriter for visualization
    pub fn set_ring_buffer(&mut self, ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>) {
        self.ring_buffer = ring_buffer;
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
        if let Some(nc) = config.get("num_channels").and_then(|v| v.as_u64()) {
            self.num_channels = nc as usize;
        }
        Ok(())
    }

    async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
        // Try to read from device if available
        if let Some(ref channels) = self.device_channels {
            match channels.filled_rx.try_recv() {
                Ok(packet) => {
                    // We have real audio from device - convert and use it

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

                    // Convert PacketBuffer to DataFrame
                    let converted_frame = packet_to_frame(&packet, self.sequence)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to convert packet to frame (format: {}, channels: {}): {}",
                            format_name, num_channels, e
                        ))?;

                    // Increment sequence for next frame
                    self.sequence += 1;

                    // Write to ring buffer for visualization if available
                    if let Some(ref rb) = self.ring_buffer {
                        if let Ok(mut writer) = rb.lock() {
                            // Extract channel data for ring buffer
                            let mut channels_data = Vec::new();
                            for ch in 0..self.num_channels {
                                if let Some(ch_data) = converted_frame.payload.get(&format!("ch{}", ch)) {
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

                    return Ok(converted_frame);
                }
                Err(_) => {
                    // No packet available - fall through to silent audio generation
                }
            }
        }

        // No device or no packet available - generate silent audio (backward compatible)
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

    async fn on_destroy(&mut self) -> Result<()> {
        // Clean up resources if needed
        self.device_channels = None;
        self.ring_buffer = None;
        Ok(())
    }
}
