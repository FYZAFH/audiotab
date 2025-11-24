use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::DebugSinkNode;
use std::sync::Arc;

#[tokio::test]
async fn test_print_passthrough() {
    let mut debug_sink = DebugSinkNode::default();
    let config = serde_json::json!({"log_level": "info"});

    debug_sink.on_create(config).await.unwrap();

    let mut df = DataFrame::new(1000, 1);
    df.payload
        .insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let result = debug_sink.process(df.clone()).await.unwrap();

    // DebugSink should pass through unchanged
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);
    assert_eq!(result.payload, df.payload);
}
