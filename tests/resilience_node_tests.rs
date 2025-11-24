use audiotab::resilience::{ResilientNode, ErrorPolicy};
use audiotab::core::{ProcessingNode, DataFrame};
use audiotab::nodes::GainNode;
use audiotab::observability::NodeMetrics;
use tokio::sync::mpsc;
use std::sync::Arc;

#[tokio::test]
async fn test_resilient_node_success() {
    let mut gain = Box::new(GainNode::default());
    gain.on_create(serde_json::json!({"gain_db": 6.0})).await.unwrap(); // +6dB = 2x gain

    let metrics = Arc::new(NodeMetrics::new("gain"));
    let mut resilient = ResilientNode::new(
        gain,
        metrics.clone(),
        ErrorPolicy::Propagate,
    );

    let (tx_in, mut rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    // Send test frame
    let mut frame = DataFrame::new(0, 0);
    frame.payload.insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0]));
    tx_in.send(frame).await.unwrap();
    drop(tx_in);

    // Run resilient node
    tokio::spawn(async move {
        while let Some(input_frame) = rx_in.recv().await {
            match resilient.process(input_frame).await {
                Ok(output) => {
                    if tx_out.send(output).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Verify output
    let output = rx_out.recv().await.unwrap();
    let result = output.payload.get("main_channel").unwrap().as_ref();
    // +6dB is 2x gain, so [1.0, 2.0] becomes [2.0, 4.0]
    assert!((result[0] - 2.0).abs() < 0.001);
    assert!((result[1] - 4.0).abs() < 0.001);

    // Verify metrics
    assert_eq!(metrics.frames_processed(), 1);
    assert_eq!(metrics.errors_count(), 0);
}
