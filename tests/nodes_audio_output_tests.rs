use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::AudioOutputNode;
use audiotab::hal::{DeviceChannels, SampleData, SampleFormat};
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_audio_output_node_creation() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 2
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::F32);
    node.on_create(config).await.unwrap();

    // Verify node is created successfully
    assert_eq!(node.sample_rate, 48000);
    assert_eq!(node.num_channels, 2);
}

#[tokio::test]
async fn test_audio_output_node_processes_frame() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 1
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::F32);
    node.on_create(config).await.unwrap();

    // Create a test DataFrame with known data
    let mut payload = HashMap::new();
    let test_samples = vec![0.1f64, 0.2, 0.3, 0.4, 0.5];
    payload.insert("ch0".to_string(), Arc::new(test_samples.clone()));

    let mut metadata = HashMap::new();
    metadata.insert("sample_rate".to_string(), "48000".to_string());

    let input_frame = DataFrame {
        timestamp: 1000000,
        sequence_id: 1,
        payload,
        metadata,
    };

    // Process the frame (should convert and send to device)
    let output_frame = node.process(input_frame).await.unwrap();

    // Verify output frame is pass-through
    assert_eq!(output_frame.sequence_id, 1);
    assert!(output_frame.payload.contains_key("ch0"));

    // Verify packet was sent to device
    let sent_packet = empty_rx.try_recv().unwrap();
    assert_eq!(sent_packet.num_channels, 1);
    assert_eq!(sent_packet.sample_rate, 48000);

    // Verify data was correctly converted (F64 -> F32)
    match sent_packet.data {
        SampleData::F32(samples) => {
            assert_eq!(samples.len(), 5);
            for (i, &expected) in test_samples.iter().enumerate() {
                assert!((samples[i] as f64 - expected).abs() < 1e-6);
            }
        }
        _ => panic!("Expected F32 data"),
    }
}

#[tokio::test]
async fn test_audio_output_node_multi_channel() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "I16",
        "num_channels": 2
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::I16);
    node.on_create(config).await.unwrap();

    // Create stereo DataFrame
    let mut payload = HashMap::new();
    payload.insert("ch0".to_string(), Arc::new(vec![0.1f64, 0.2, 0.3]));
    payload.insert("ch1".to_string(), Arc::new(vec![0.4f64, 0.5, 0.6]));

    let mut metadata = HashMap::new();
    metadata.insert("sample_rate".to_string(), "48000".to_string());

    let input_frame = DataFrame {
        timestamp: 2000000,
        sequence_id: 2,
        payload,
        metadata,
    };

    let _output_frame = node.process(input_frame).await.unwrap();

    // Verify packet was sent
    let sent_packet = empty_rx.try_recv().unwrap();
    assert_eq!(sent_packet.num_channels, 2);

    // Verify interleaving and conversion
    match sent_packet.data {
        SampleData::I16(samples) => {
            // Should be interleaved: [L0, R0, L1, R1, L2, R2]
            assert_eq!(samples.len(), 6);

            // Check approximate values (allowing for I16 quantization)
            assert!((samples[0] as f64 / 32768.0 - 0.1).abs() < 0.01);  // L0
            assert!((samples[1] as f64 / 32768.0 - 0.4).abs() < 0.01);  // R0
            assert!((samples[2] as f64 / 32768.0 - 0.2).abs() < 0.01);  // L1
            assert!((samples[3] as f64 / 32768.0 - 0.5).abs() < 0.01);  // R1
            assert!((samples[4] as f64 / 32768.0 - 0.3).abs() < 0.01);  // L2
            assert!((samples[5] as f64 / 32768.0 - 0.6).abs() < 0.01);  // R2
        }
        _ => panic!("Expected I16 data"),
    }
}

#[tokio::test]
async fn test_audio_output_node_empty_frame() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 1
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::F32);
    node.on_create(config).await.unwrap();

    // Create empty DataFrame
    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should return frame without sending to device
    assert_eq!(output_frame.sequence_id, 0);

    // No packet should be sent for empty frame
    assert!(empty_rx.try_recv().is_err());
}

#[tokio::test]
async fn test_audio_output_node_different_formats() {
    // Test I32
    {
        let (_filled_tx, filled_rx) = unbounded();
        let (empty_tx, empty_rx) = unbounded();

        let channels = DeviceChannels {
            filled_rx,
            empty_tx,
        };

        let mut node = AudioOutputNode::new(channels, SampleFormat::I32);
        let config = serde_json::json!({
            "sample_rate": 48000,
            "format": "I32",
            "num_channels": 1
        });
        node.on_create(config).await.unwrap();

        let mut payload = HashMap::new();
        payload.insert("ch0".to_string(), Arc::new(vec![0.5f64, -0.5]));
        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), "48000".to_string());
        let frame = DataFrame { timestamp: 0, sequence_id: 1, payload, metadata };

        node.process(frame).await.unwrap();
        let packet = empty_rx.try_recv().unwrap();
        match packet.data {
            SampleData::I32(samples) => {
                assert_eq!(samples.len(), 2);
            }
            _ => panic!("Expected I32 data"),
        }
    }

    // Test F64
    {
        let (_filled_tx, filled_rx) = unbounded();
        let (empty_tx, empty_rx) = unbounded();

        let channels = DeviceChannels {
            filled_rx,
            empty_tx,
        };

        let mut node = AudioOutputNode::new(channels, SampleFormat::F64);
        let config = serde_json::json!({
            "sample_rate": 48000,
            "format": "F64",
            "num_channels": 1
        });
        node.on_create(config).await.unwrap();

        let mut payload = HashMap::new();
        payload.insert("ch0".to_string(), Arc::new(vec![0.7f64, -0.3]));
        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), "48000".to_string());
        let frame = DataFrame { timestamp: 0, sequence_id: 1, payload, metadata };

        node.process(frame).await.unwrap();
        let packet = empty_rx.try_recv().unwrap();
        match packet.data {
            SampleData::F64(samples) => {
                assert_eq!(samples.len(), 2);
                assert!((samples[0] - 0.7).abs() < 1e-10);
                assert!((samples[1] - (-0.3)).abs() < 1e-10);
            }
            _ => panic!("Expected F64 data"),
        }
    }

    // Test U8
    {
        let (_filled_tx, filled_rx) = unbounded();
        let (empty_tx, empty_rx) = unbounded();

        let channels = DeviceChannels {
            filled_rx,
            empty_tx,
        };

        let mut node = AudioOutputNode::new(channels, SampleFormat::U8);
        let config = serde_json::json!({
            "sample_rate": 48000,
            "format": "U8",
            "num_channels": 1
        });
        node.on_create(config).await.unwrap();

        let mut payload = HashMap::new();
        payload.insert("ch0".to_string(), Arc::new(vec![0.0f64, 0.5, -0.5]));
        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), "48000".to_string());
        let frame = DataFrame { timestamp: 0, sequence_id: 1, payload, metadata };

        node.process(frame).await.unwrap();
        let packet = empty_rx.try_recv().unwrap();
        match packet.data {
            SampleData::U8(samples) => {
                assert_eq!(samples.len(), 3);
                // 0.0 -> 128, 0.5 -> 192, -0.5 -> 64
                assert_eq!(samples[0], 128);
            }
            _ => panic!("Expected U8 data"),
        }
    }
}

#[tokio::test]
async fn test_audio_output_node_sequence_passthrough() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 1
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::F32);
    node.on_create(config).await.unwrap();

    // Process multiple frames with different sequence IDs
    for i in 1..=3 {
        let mut payload = HashMap::new();
        payload.insert("ch0".to_string(), Arc::new(vec![i as f64]));
        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), "48000".to_string());
        let frame = DataFrame {
            timestamp: i * 1000000,
            sequence_id: i,
            payload,
            metadata,
        };

        let output_frame = node.process(frame).await.unwrap();
        assert_eq!(output_frame.sequence_id, i);
    }
}

#[tokio::test]
async fn test_audio_output_node_timestamp_preservation() {
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 1
    });

    let mut node = AudioOutputNode::new(channels, SampleFormat::F32);
    node.on_create(config).await.unwrap();

    let test_timestamp = 5000000u64;
    let mut payload = HashMap::new();
    payload.insert("ch0".to_string(), Arc::new(vec![0.5f64]));
    let mut metadata = HashMap::new();
    metadata.insert("sample_rate".to_string(), "48000".to_string());
    let frame = DataFrame {
        timestamp: test_timestamp,
        sequence_id: 1,
        payload,
        metadata,
    };

    node.process(frame).await.unwrap();
    let packet = empty_rx.try_recv().unwrap();
    assert_eq!(packet.timestamp, Some(test_timestamp));
}
