use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::Gain;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_gain_multiplication() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 2.0});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload
        .insert("main_channel".to_string(), vec![1.0, 2.0, 3.0]);

    let result = gain.process(df).await.unwrap();
    assert_eq!(
        result.payload.get("main_channel").unwrap(),
        &vec![2.0, 4.0, 6.0]
    );
}

#[tokio::test]
async fn test_gain_attenuation() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 0.5});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload
        .insert("main_channel".to_string(), vec![2.0, 4.0, 6.0]);

    let result = gain.process(df).await.unwrap();
    assert_eq!(
        result.payload.get("main_channel").unwrap(),
        &vec![1.0, 2.0, 3.0]
    );
}

#[tokio::test]
async fn test_gain_streaming() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 2.0});
    gain.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        gain.run(rx_in, tx_out).await
    });

    // Send 2 frames
    let mut df1 = DataFrame::new(0, 0);
    df1.payload.insert("main_channel".to_string(), vec![1.0, 2.0]);
    tx_in.send(df1).await.unwrap();

    let mut df2 = DataFrame::new(1000, 1);
    df2.payload.insert("main_channel".to_string(), vec![3.0, 4.0]);
    tx_in.send(df2).await.unwrap();

    drop(tx_in);

    // Verify results
    let result1 = rx_out.recv().await.unwrap();
    assert_eq!(result1.payload.get("main_channel").unwrap(), &vec![2.0, 4.0]);

    let result2 = rx_out.recv().await.unwrap();
    assert_eq!(result2.payload.get("main_channel").unwrap(), &vec![6.0, 8.0]);

    handle.await.unwrap().unwrap();
}
