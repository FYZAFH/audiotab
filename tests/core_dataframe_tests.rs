use audiotab::core::DataFrame;

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
    df.payload.insert("channel1".to_string(), vec![1.0, 2.0, 3.0]);

    assert_eq!(df.payload.get("channel1").unwrap(), &vec![1.0, 2.0, 3.0]);
}
