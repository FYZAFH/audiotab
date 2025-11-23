use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use crate::core::{ProcessingNode, DataFrame};

pub struct Gain {
    gain: f64,
}

impl Default for Gain {
    fn default() -> Self {
        Self::new()
    }
}

impl Gain {
    pub fn new() -> Self {
        Self { gain: 1.0 }
    }
}

#[async_trait]
impl ProcessingNode for Gain {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(g) = config["gain"].as_f64() {
            self.gain = g;
        }
        Ok(())
    }

    async fn process(&self, mut input: DataFrame) -> Result<DataFrame> {
        // Apply gain to main_channel if it exists
        if let Some(data) = input.payload.get_mut("main_channel") {
            for sample in data.iter_mut() {
                *sample *= self.gain;
            }
        }
        Ok(input)
    }
}
