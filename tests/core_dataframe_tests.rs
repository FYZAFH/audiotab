use audiotab::core::DataFrame;
use std::sync::Arc;

#[test]
fn test_dataframe_creation() {
    let df = DataFrame::new(1000, 1);
    assert_eq!(df.timestamp, 1000);
    assert_eq!(df.sequence_id, 1);
    assert!(df.payload.is_empty());
    assert!(df.metadata.is_empty());
}

#[test]
fn test_dataframe_with_data() {
    let mut df = DataFrame::new(2000, 2);
    df.payload
        .insert("channel1".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    assert_eq!(df.payload.get("channel1").unwrap().as_ref(), &vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_dataframe_zero_copy_clone() {
    let mut frame = DataFrame::new(1000, 1);
    frame.payload.insert("channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let _cloned = frame.clone();

    // Both should share the same Arc
    assert_eq!(
        Arc::strong_count(frame.payload.get("channel").unwrap()),
        2
    );
}
