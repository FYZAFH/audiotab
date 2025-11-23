use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use std::f64::consts::PI;
use crate::core::{ProcessingNode, DataFrame};

pub struct SineGenerator {
    frequency: f64,
    sample_rate: f64,
    frame_size: usize,
    phase: f64,  // Current phase for continuous generation
}

impl Default for SineGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SineGenerator {
    pub fn new() -> Self {
        Self {
            frequency: 440.0,
            sample_rate: 48000.0,
            frame_size: 1024,
            phase: 0.0,
        }
    }
}

#[async_trait]
impl ProcessingNode for SineGenerator {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(freq) = config["frequency"].as_f64() {
            self.frequency = freq;
        }
        if let Some(sr) = config["sample_rate"].as_f64() {
            self.sample_rate = sr;
        }
        if let Some(size) = config["frame_size"].as_u64() {
            self.frame_size = size as usize;
        }
        Ok(())
    }

    async fn process(&self, mut input: DataFrame) -> Result<DataFrame> {
        let mut samples = Vec::with_capacity(self.frame_size);
        let phase_increment = 2.0 * PI * self.frequency / self.sample_rate;

        for i in 0..self.frame_size {
            let phase = self.phase + (i as f64) * phase_increment;
            samples.push(phase.sin());
        }

        input.payload.insert("main_channel".to_string(), samples);
        Ok(input)
    }
}
