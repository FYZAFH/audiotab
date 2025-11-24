use crate::core::{DataFrame, ProcessingNode};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

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
        if let Some(data) = input.payload.get("main_channel") {
            let amplified: Vec<f64> = data.iter().map(|&x| x * self.gain).collect();
            input.payload.insert("main_channel".to_string(), Arc::new(amplified));
        }
        Ok(input)
    }

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(mut frame) = rx.recv().await {
            if let Some(data) = frame.payload.get("main_channel") {
                // Clone Arc data, apply gain, wrap in new Arc
                let amplified: Vec<f64> = data.iter().map(|&x| x * self.gain).collect();
                frame.payload.insert("main_channel".to_string(), Arc::new(amplified));
            }

            if tx.send(frame).await.is_err() {
                break; // Downstream closed
            }
        }

        Ok(())
    }
}
