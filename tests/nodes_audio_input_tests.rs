use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::AudioInputNode;
use audiotab::hal::{DeviceChannels, PacketBuffer, SampleData};
use audiotab::visualization::RingBufferWriter;
use crossbeam_channel::unbounded;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_audio_input_node_creation() {
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

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    // Verify node is created successfully
    assert_eq!(node.sample_rate, 48000);
    assert_eq!(node.num_channels, 2);
}

#[tokio::test]
async fn test_audio_input_node_processes_packet() {
    let (filled_tx, filled_rx) = unbounded();
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

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    // Create a test packet with known data
    let test_samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
    let packet = PacketBuffer {
        data: SampleData::F32(test_samples.clone()),
        sample_rate: 48000,
        num_channels: 1,
        timestamp: Some(1000000),
    };

    // Send packet to the node
    filled_tx.send(packet).unwrap();

    // Process a frame (should read the packet)
    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Verify output contains converted data
    assert!(output_frame.payload.contains_key("ch0"));
    let ch0_data = output_frame.payload.get("ch0").unwrap();
    assert_eq!(ch0_data.len(), 5);

    // Verify values are correctly converted (F32 -> F64)
    for (i, &expected) in test_samples.iter().enumerate() {
        assert!((ch0_data[i] - expected as f64).abs() < 1e-6);
    }

    // Verify packet was returned to device
    let returned_packet = empty_rx.try_recv().unwrap();
    assert_eq!(returned_packet.num_channels, 1);
}

#[tokio::test]
async fn test_audio_input_node_multi_channel() {
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "I16",
        "num_channels": 2
    });

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    // Create interleaved stereo packet: [L0, R0, L1, R1, L2, R2]
    let test_samples = vec![
        1000i16, 2000i16,  // Frame 0
        3000i16, 4000i16,  // Frame 1
        5000i16, 6000i16,  // Frame 2
    ];

    let packet = PacketBuffer {
        data: SampleData::I16(test_samples),
        sample_rate: 48000,
        num_channels: 2,
        timestamp: Some(2000000),
    };

    filled_tx.send(packet).unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Verify both channels exist
    assert!(output_frame.payload.contains_key("ch0"));
    assert!(output_frame.payload.contains_key("ch1"));

    let ch0 = output_frame.payload.get("ch0").unwrap();
    let ch1 = output_frame.payload.get("ch1").unwrap();

    // Each channel should have 3 samples (de-interleaved)
    assert_eq!(ch0.len(), 3);
    assert_eq!(ch1.len(), 3);

    // Verify de-interleaving and normalization
    assert!((ch0[0] - (1000.0 / 32768.0)).abs() < 1e-6);
    assert!((ch0[1] - (3000.0 / 32768.0)).abs() < 1e-6);
    assert!((ch0[2] - (5000.0 / 32768.0)).abs() < 1e-6);

    assert!((ch1[0] - (2000.0 / 32768.0)).abs() < 1e-6);
    assert!((ch1[1] - (4000.0 / 32768.0)).abs() < 1e-6);
    assert!((ch1[2] - (6000.0 / 32768.0)).abs() < 1e-6);
}

#[tokio::test]
async fn test_audio_input_node_with_ring_buffer() {
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    // Create ring buffer writer
    let ring_buffer_path = "/tmp/test_audio_input_ringbuf";
    let _ = std::fs::remove_file(ring_buffer_path);
    let ring_buffer = RingBufferWriter::new(ring_buffer_path, 48000, 2, 1).unwrap();
    let ring_buffer_arc = Arc::new(Mutex::new(ring_buffer));

    let config = serde_json::json!({
        "sample_rate": 48000,
        "format": "F32",
        "num_channels": 2
    });

    let mut node = AudioInputNode::new(channels, Some(ring_buffer_arc.clone()));
    node.on_create(config).await.unwrap();

    // Create test packet
    let test_samples = vec![
        0.1f32, 0.2f32,  // Frame 0: L=0.1, R=0.2
        0.3f32, 0.4f32,  // Frame 1: L=0.3, R=0.4
        0.5f32, 0.6f32,  // Frame 2: L=0.5, R=0.6
    ];

    let packet = PacketBuffer {
        data: SampleData::F32(test_samples),
        sample_rate: 48000,
        num_channels: 2,
        timestamp: Some(3000000),
    };

    filled_tx.send(packet).unwrap();

    let input_frame = DataFrame::new(0, 0);
    let _output_frame = node.process(input_frame).await.unwrap();

    // Verify ring buffer was updated
    let rb = ring_buffer_arc.lock().unwrap();
    let seq = rb.get_write_sequence();
    assert_eq!(seq, 1);

    // Cleanup
    drop(rb);
    drop(ring_buffer_arc);
    std::fs::remove_file(ring_buffer_path).unwrap();
}

#[tokio::test]
async fn test_audio_input_node_no_packet_available() {
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

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    // Don't send any packet - node should handle gracefully
    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should return frame with empty payload or previous frame
    // (implementation detail - can choose to return empty or cached data)
    // Sequence should increment even for empty frames to maintain consistency
    assert_eq!(output_frame.sequence_id, 1);
}

#[tokio::test]
async fn test_audio_input_node_sequence_increment() {
    let (filled_tx, filled_rx) = unbounded();
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

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    // Send multiple packets
    for i in 0..3 {
        let packet = PacketBuffer {
            data: SampleData::F32(vec![i as f32; 10]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(i * 1000000),
        };
        filled_tx.send(packet).unwrap();

        let input_frame = DataFrame::new(0, i);
        let output_frame = node.process(input_frame).await.unwrap();
        assert_eq!(output_frame.sequence_id, i + 1);
    }
}

#[tokio::test]
async fn test_audio_input_node_metadata() {
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 96000,
        "format": "F64",
        "num_channels": 1
    });

    let mut node = AudioInputNode::new(channels, None);
    node.on_create(config).await.unwrap();

    let packet = PacketBuffer {
        data: SampleData::F64(vec![1.0, 2.0, 3.0]),
        sample_rate: 96000,
        num_channels: 1,
        timestamp: Some(5000000),
    };

    filled_tx.send(packet).unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Verify metadata contains sample rate
    assert!(output_frame.metadata.contains_key("sample_rate"));
    assert_eq!(output_frame.metadata.get("sample_rate").unwrap(), "96000");
}
