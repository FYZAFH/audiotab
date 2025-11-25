use async_trait::async_trait;
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::hal::{Device, DeviceChannels, DeviceCapabilities, PacketBuffer, SampleFormat};

pub struct AudioDevice {
    device_name: String,
    filled_rx: Receiver<PacketBuffer>,
    empty_tx: Sender<PacketBuffer>,
    is_streaming: Arc<AtomicBool>,
    capabilities: DeviceCapabilities,
}

impl AudioDevice {
    pub fn new(
        device_name: String,
        _sample_rate: u64,
        format: SampleFormat,
        buffer_size: usize,
        num_channels: usize,
    ) -> Result<Self> {
        let (_filled_tx, filled_rx) = bounded(2);  // Double buffer
        let (empty_tx, _empty_rx) = bounded(2);

        // Pre-allocate ping-pong buffers
        for _ in 0..2 {
            let buffer = PacketBuffer::new(format, buffer_size, num_channels);
            empty_tx.send(buffer).map_err(|e| anyhow::anyhow!("Failed to send buffer: {}", e))?;
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
            filled_rx,
            empty_tx,
            is_streaming: Arc::new(AtomicBool::new(false)),
            capabilities,
        })
    }
}

#[async_trait]
impl Device for AudioDevice {
    async fn start(&mut self) -> Result<()> {
        self.is_streaming.store(true, Ordering::Relaxed);
        // CPAL stream start will be implemented in next task
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
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
