use audiotab::resilience::{ResilientNode, ErrorPolicy};
use audiotab::core::{ProcessingNode, DataFrame};
use audiotab::nodes::Gain;
use audiotab::observability::NodeMetrics;
use tokio::sync::mpsc;
use std::sync::Arc;

#[tokio::test]
async fn test_resilient_node_success() {
    let gain = Box::new(Gain::new());
    let mut gain_configured = gain;
    gain_configured.on_create(serde_json::json!({"gain": 2.0})).await.unwrap();

    let metrics = Arc::new(NodeMetrics::new("gain"));
    let resilient = ResilientNode::new(
        gain_configured,
        metrics.clone(),
        ErrorPolicy::Propagate,
    );

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    // Send test frame
    let mut frame = DataFrame::new(0, 0);
    frame.payload.insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0]));
    tx_in.send(frame).await.unwrap();
    drop(tx_in);

    // Run resilient node
    tokio::spawn(async move {
        resilient.run(rx_in, tx_out).await.unwrap();
    });

    // Verify output
    let output = rx_out.recv().await.unwrap();
    assert_eq!(output.payload.get("main_channel").unwrap().as_ref(), &vec![2.0, 4.0]);

    // Verify metrics
    assert_eq!(metrics.frames_processed(), 1);
    assert_eq!(metrics.errors_count(), 0);
}
