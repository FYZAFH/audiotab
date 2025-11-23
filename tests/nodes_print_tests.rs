use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::Print;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_print_passthrough() {
    let mut print = Print::new();
    let config = serde_json::json!({"label": "Test"});

    print.on_create(config).await.unwrap();

    let mut df = DataFrame::new(1000, 1);
    df.payload
        .insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let result = print.process(df.clone()).await.unwrap();

    // Print should pass through unchanged
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);
    assert_eq!(result.payload, df.payload);
}

#[tokio::test]
async fn test_print_streaming() {
    let mut print = Print::new();
    let config = serde_json::json!({"label": "StreamTest"});
    print.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        print.run(rx_in, tx_out).await
    });

    let mut df = DataFrame::new(5000, 5);
    df.payload.insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));
    tx_in.send(df.clone()).await.unwrap();

    drop(tx_in);

    let result = rx_out.recv().await.unwrap();
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);

    handle.await.unwrap().unwrap();
}
