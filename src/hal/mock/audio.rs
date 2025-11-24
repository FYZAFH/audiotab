use crate::hal::{DeviceSource, DeviceState};
use crate::core::DataFrame;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::f64::consts::PI;

pub struct SimulatedAudioSource {
    state: DeviceState,
    frequency: f64,
    sample_rate: f64,
    amplitude: f64,
    samples_per_frame: usize,
    phase: f64,
    frame_counter: u64,
}

impl SimulatedAudioSource {
    pub fn new() -> Self {
        Self {
            state: DeviceState::Unopened,
            frequency: 1000.0,
            sample_rate: 48000.0,
            amplitude: 1.0,
            samples_per_frame: 1024,
            phase: 0.0,
            frame_counter: 0,
        }
    }

    fn generate_samples(&mut self) -> Vec<f64> {
        let mut samples = Vec::with_capacity(self.samples_per_frame);
        let delta_phase = 2.0 * PI * self.frequency / self.sample_rate;

        for _ in 0..self.samples_per_frame {
            samples.push(self.amplitude * self.phase.sin());
            self.phase += delta_phase;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }

        samples
    }
}

#[async_trait]
impl DeviceSource for SimulatedAudioSource {
    async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }

        if let Some(freq) = config["frequency"].as_f64() {
            self.frequency = freq;
        }
        if let Some(sr) = config["sample_rate"].as_f64() {
            self.sample_rate = sr;
        }
        if let Some(amp) = config["amplitude"].as_f64() {
            self.amplitude = amp;
        }
        if let Some(spf) = config["samples_per_frame"].as_u64() {
            self.samples_per_frame = spf as usize;
        }

        Ok(())
    }

    async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.state = DeviceState::Opened;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.state = DeviceState::Running;
        self.phase = 0.0; // Reset phase on start
        self.frame_counter = 0;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<DataFrame> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not running"));
        }

        let samples = self.generate_samples();
        let timestamp = (self.frame_counter * self.samples_per_frame as u64 * 1_000_000)
            / self.sample_rate as u64;

        let mut frame = DataFrame::new(timestamp, self.frame_counter);
        frame.payload.insert("audio".to_string(), Arc::new(samples));
        frame.metadata.insert("sample_rate".to_string(), self.sample_rate.to_string());
        frame.metadata.insert("frequency".to_string(), self.frequency.to_string());

        self.frame_counter += 1;

        Ok(frame)
    }

    async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(());
        }
        self.state = DeviceState::Stopped;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    fn state(&self) -> DeviceState {
        self.state.clone()
    }
}
