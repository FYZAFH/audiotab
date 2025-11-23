use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use crate::core::{ProcessingNode, DataFrame};

pub struct Print {
    label: String,
}

impl Print {
    pub fn new() -> Self {
        Self {
            label: "Output".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for Print {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(label) = config["label"].as_str() {
            self.label = label.to_string();
        }
        Ok(())
    }

    async fn process(&self, input: DataFrame) -> Result<DataFrame> {
        println!("[{}] Frame #{} @ {}μs", self.label, input.sequence_id, input.timestamp);

        for (channel, data) in &input.payload {
            let stats = if !data.is_empty() {
                let sum: f64 = data.iter().sum();
                let mean = sum / data.len() as f64;
                let rms = (data.iter().map(|x| x * x).sum::<f64>() / data.len() as f64).sqrt();
                format!("len={}, mean={:.4}, rms={:.4}", data.len(), mean, rms)
            } else {
                "empty".to_string()
            };
            println!("  {}: {}", channel, stats);
        }

        Ok(input)
    }
}
