use audiotab::core::{DataFrame, ProcessingNode};
use anyhow::Result;
use async_trait::async_trait;

struct DummyNode {
    multiplier: f64,
}

#[async_trait]
impl ProcessingNode for DummyNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        self.multiplier = config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(())
    }

    async fn process(&self, input: DataFrame) -> Result<DataFrame> {
        let mut output = input.clone();
        if let Some(data) = output.payload.get_mut("test") {
            for value in data.iter_mut() {
                *value *= self.multiplier;
            }
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
    df.payload.insert("test".to_string(), vec![1.0, 2.0, 3.0]);

    let result = node.process(df).await.unwrap();
    assert_eq!(result.payload.get("test").unwrap(), &vec![2.0, 4.0, 6.0]);
}
