use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::Gain;

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
