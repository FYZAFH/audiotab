use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::GainNode;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_gain_multiplication() {
    let mut gain = GainNode::default();
    // +6 dB is approximately 2x gain
    let config = serde_json::json!({"gain_db": 6.0206});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload
        .insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let result = gain.process(df).await.unwrap();
    let output = result.payload.get("main_channel").unwrap().as_ref();
    // Check approximate values (2x gain)
    assert!((output[0] - 2.0).abs() < 0.001);
    assert!((output[1] - 4.0).abs() < 0.001);
    assert!((output[2] - 6.0).abs() < 0.001);
}

#[tokio::test]
async fn test_gain_attenuation() {
    let mut gain = GainNode::default();
    // -6 dB is approximately 0.5x gain
    let config = serde_json::json!({"gain_db": -6.0206});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload
        .insert("main_channel".to_string(), Arc::new(vec![2.0, 4.0, 6.0]));

    let result = gain.process(df).await.unwrap();
    let output = result.payload.get("main_channel").unwrap().as_ref();
    // Check approximate values (0.5x gain)
    assert!((output[0] - 1.0).abs() < 0.001);
    assert!((output[1] - 2.0).abs() < 0.001);
    assert!((output[2] - 3.0).abs() < 0.001);
}

#[tokio::test]
async fn test_gain_streaming() {
    let mut gain = GainNode::default();
    let config = serde_json::json!({"gain_db": 6.0206}); // ~2x
    gain.on_create(config).await.unwrap();

    let (tx_in, mut rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        while let Some(frame) = rx_in.recv().await {
            match gain.process(frame).await {
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

    // Send 2 frames
    let mut df1 = DataFrame::new(0, 0);
    df1.payload.insert("main_channel".to_string(), Arc::new(vec![1.0, 2.0]));
    tx_in.send(df1).await.unwrap();

    let mut df2 = DataFrame::new(1000, 1);
    df2.payload.insert("main_channel".to_string(), Arc::new(vec![3.0, 4.0]));
    tx_in.send(df2).await.unwrap();

    drop(tx_in);

    // Verify results
    let result1 = rx_out.recv().await.unwrap();
    let output1 = result1.payload.get("main_channel").unwrap().as_ref();
    assert!((output1[0] - 2.0).abs() < 0.001);
    assert!((output1[1] - 4.0).abs() < 0.001);

    let result2 = rx_out.recv().await.unwrap();
    let output2 = result2.payload.get("main_channel").unwrap().as_ref();
    assert!((output2[0] - 6.0).abs() < 0.001);
    assert!((output2[1] - 8.0).abs() < 0.001);

    handle.await.unwrap().unwrap();
}
