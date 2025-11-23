use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::Print;

#[tokio::test]
async fn test_print_passthrough() {
    let mut print = Print::new();
    let config = serde_json::json!({"label": "Test"});

    print.on_create(config).await.unwrap();

    let mut df = DataFrame::new(1000, 1);
    df.payload
        .insert("main_channel".to_string(), vec![1.0, 2.0, 3.0]);

    let result = print.process(df.clone()).await.unwrap();

    // Print should pass through unchanged
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);
    assert_eq!(result.payload, df.payload);
}
