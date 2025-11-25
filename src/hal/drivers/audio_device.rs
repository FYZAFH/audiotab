use async_trait::async_trait;
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crate::hal::{Device, DeviceChannels, DeviceCapabilities, PacketBuffer, SampleData, SampleFormat};

// Wrapper to make Stream Send (it's thread-safe, just not marked Send on all platforms)
struct SendStream(Stream);
unsafe impl Send for SendStream {}

pub struct AudioDevice {
    device_name: String,
    sample_rate: u64,
    format: SampleFormat,
    buffer_size: usize,
    num_channels: usize,
    filled_tx: Sender<PacketBuffer>,
    filled_rx: Receiver<PacketBuffer>,
    empty_tx: Sender<PacketBuffer>,
    empty_rx: Receiver<PacketBuffer>,
    is_streaming: Arc<AtomicBool>,
    capabilities: DeviceCapabilities,
    stream: Option<SendStream>,
}

impl AudioDevice {
    pub fn new(
        device_name: String,
        sample_rate: u64,
        format: SampleFormat,
        buffer_size: usize,
        num_channels: usize,
    ) -> Result<Self> {
        let (filled_tx, filled_rx) = bounded(2);
        let (empty_tx, empty_rx) = bounded(2);

        // Pre-allocate buffers
        for _ in 0..2 {
            let buffer = PacketBuffer::new(format, buffer_size, num_channels);
            empty_tx.send(buffer)
                .map_err(|e| anyhow::anyhow!("Failed to send buffer: {}", e))?;
        }

        let capabilities = DeviceCapabilities {
            can_input: true,
            can_output: false,
            supported_formats: vec![SampleFormat::F32, SampleFormat::I16],
            supported_sample_rates: vec![44100, 48000, 96000, 192000],
            max_channels: 32,
        };

        Ok(Self {
            device_name,
            sample_rate,
            format,
            buffer_size,
            num_channels,
            filled_tx,
            filled_rx,
            empty_tx,
            empty_rx,
            is_streaming: Arc::new(AtomicBool::new(false)),
            capabilities,
            stream: None,
        })
    }

    fn start_cpal_stream(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device"))?;

        let config = StreamConfig {
            channels: self.num_channels as u16,
            sample_rate: cpal::SampleRate(self.sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(self.buffer_size as u32),
        };

        let empty_rx = self.empty_rx.clone();
        let filled_tx = self.filled_tx.clone();
        let num_channels = self.num_channels;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Try to get empty buffer
                if let Ok(mut buffer) = empty_rx.try_recv() {
                    // Copy audio data
                    if let SampleData::F32(ref mut samples) = buffer.data {
                        let copy_len = data.len().min(samples.len());
                        samples[..copy_len].copy_from_slice(&data[..copy_len]);
                        buffer.num_channels = num_channels;
                    }

                    // Send filled buffer
                    let _ = filled_tx.try_send(buffer);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(SendStream(stream));

        Ok(())
    }
}

#[async_trait]
impl Device for AudioDevice {
    async fn start(&mut self) -> Result<()> {
        self.start_cpal_stream()?;
        self.is_streaming.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.stream = None;  // Drops stream, stops playback
        self.is_streaming.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn get_channels(&mut self) -> DeviceChannels {
        DeviceChannels {
            filled_rx: self.filled_rx.clone(),
            empty_tx: self.empty_tx.clone(),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.capabilities.clone()
    }

    fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }
}
