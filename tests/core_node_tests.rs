use anyhow::{anyhow, Result};
use async_trait::async_trait;
use audiotab::core::{DataFrame, ProcessingNode};
use std::sync::Arc;
use tokio::sync::mpsc;

struct DummyNode {
    multiplier: f64,
}

#[async_trait]
impl ProcessingNode for DummyNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        self.multiplier = config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(())
    }

    async fn process(&mut self, input: DataFrame) -> Result<DataFrame> {
        let mut output = input.clone();
        if let Some(data) = output.payload.get("test") {
            let multiplied: Vec<f64> = data.iter().map(|&x| x * self.multiplier).collect();
            output.payload.insert("test".to_string(), Arc::new(multiplied));
        }
        Ok(output)
    }
}

#[tokio::test]
async fn test_node_process() {
    let mut node = DummyNode { multiplier: 1.0 };
    let config = serde_json::json!({"multiplier": 2.0});

    node.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload.insert("test".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let result = node.process(df).await.unwrap();
    assert_eq!(result.payload.get("test").unwrap().as_ref(), &vec![2.0, 4.0, 6.0]);
}

struct StreamingDummyNode {
    multiplier: f64,
}

#[async_trait]
impl ProcessingNode for StreamingDummyNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        self.multiplier = config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(())
    }

    async fn process(&mut self, mut input: DataFrame) -> Result<DataFrame> {
        if let Some(data) = input.payload.get("test") {
            let multiplied: Vec<f64> = data.iter().map(|&x| x * self.multiplier).collect();
            input.payload.insert("test".to_string(), Arc::new(multiplied));
        }
        Ok(input)
    }
}

#[tokio::test]
async fn test_node_streaming() {
    let mut node = StreamingDummyNode { multiplier: 2.0 };

    let (tx_in, mut rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    // Spawn node task
    let handle = tokio::spawn(async move {
        while let Some(frame) = rx_in.recv().await {
            match node.process(frame).await {
                Ok(output) => {
                    if tx_out.send(output).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Send frames
    let mut df1 = DataFrame::new(0, 0);
    df1.payload.insert("test".to_string(), Arc::new(vec![1.0, 2.0]));
    tx_in.send(df1).await.unwrap();

    let mut df2 = DataFrame::new(1000, 1);
    df2.payload.insert("test".to_string(), Arc::new(vec![3.0, 4.0]));
    tx_in.send(df2).await.unwrap();

    drop(tx_in); // Close channel to terminate node

    // Receive results
    let result1 = rx_out.recv().await.unwrap();
    assert_eq!(result1.payload.get("test").unwrap().as_ref(), &vec![2.0, 4.0]);

    let result2 = rx_out.recv().await.unwrap();
    assert_eq!(result2.payload.get("test").unwrap().as_ref(), &vec![6.0, 8.0]);

    handle.await.unwrap().unwrap();
}
