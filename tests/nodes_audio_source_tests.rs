use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::AudioSourceNode;
use audiotab::hal::{DeviceChannels, PacketBuffer, SampleData};
use audiotab::visualization::RingBufferWriter;
use crossbeam_channel::unbounded;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_audio_source_node_default_silent() {
    // Test that default AudioSourceNode generates silent audio
    let config = serde_json::json!({
        "sample_rate": 48000,
        "buffer_size": 1024
    });

    let mut node = AudioSourceNode::default();
    node.on_create(config).await.unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should have main_channel with silent audio
    assert!(output_frame.payload.contains_key("main_channel"));
    let main_channel = output_frame.payload.get("main_channel").unwrap();
    assert_eq!(main_channel.len(), 1024);

    // All samples should be zero (silent)
    for &sample in main_channel.iter() {
        assert_eq!(sample, 0.0);
    }
}

#[tokio::test]
async fn test_audio_source_node_with_device_channels() {
    // Test that AudioSourceNode uses real device when channels are provided
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    // Create test packet with known data
    let test_samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
    let packet = PacketBuffer {
        data: SampleData::F32(test_samples.clone()),
        sample_rate: 48000,
        num_channels: 1,
        timestamp: Some(1000000),
    };

    filled_tx.send(packet).unwrap();

    let config = serde_json::json!({
        "sample_rate": 48000,
        "buffer_size": 5
    });

    let mut node = AudioSourceNode::with_device(channels, None);
    node.on_create(config).await.unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should have ch0 with real audio data
    assert!(output_frame.payload.contains_key("ch0"));
    let ch0_data = output_frame.payload.get("ch0").unwrap();
    assert_eq!(ch0_data.len(), 5);

    // Verify values are correctly converted
    for (i, &expected) in test_samples.iter().enumerate() {
        assert!((ch0_data[i] - expected as f64).abs() < 1e-6);
    }

    // Verify packet was returned to device
    let returned_packet = empty_rx.try_recv().unwrap();
    assert_eq!(returned_packet.num_channels, 1);
}

#[tokio::test]
async fn test_audio_source_node_fallback_to_silent_when_no_packet() {
    // Test that node falls back to silent audio when no packet is available
    let (_filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let config = serde_json::json!({
        "sample_rate": 48000,
        "buffer_size": 512
    });

    let mut node = AudioSourceNode::with_device(channels, None);
    node.on_create(config).await.unwrap();

    // Don't send any packet
    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should fall back to silent audio (main_channel)
    assert!(output_frame.payload.contains_key("main_channel"));
    let main_channel = output_frame.payload.get("main_channel").unwrap();
    assert_eq!(main_channel.len(), 512);

    // All samples should be zero
    for &sample in main_channel.iter() {
        assert_eq!(sample, 0.0);
    }
}

#[tokio::test]
async fn test_audio_source_node_multi_channel_device() {
    // Test that AudioSourceNode handles multi-channel device input
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    // Create interleaved stereo packet
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

    let config = serde_json::json!({
        "sample_rate": 48000,
        "num_channels": 2
    });

    let mut node = AudioSourceNode::with_device(channels, None);
    node.on_create(config).await.unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should have both channels
    assert!(output_frame.payload.contains_key("ch0"));
    assert!(output_frame.payload.contains_key("ch1"));

    let ch0 = output_frame.payload.get("ch0").unwrap();
    let ch1 = output_frame.payload.get("ch1").unwrap();

    // Each channel should have 3 samples
    assert_eq!(ch0.len(), 3);
    assert_eq!(ch1.len(), 3);

    // Verify de-interleaving and normalization
    assert!((ch0[0] - (1000.0 / 32768.0)).abs() < 1e-6);
    assert!((ch1[0] - (2000.0 / 32768.0)).abs() < 1e-6);
}

#[tokio::test]
async fn test_audio_source_node_with_ring_buffer() {
    // Test that AudioSourceNode writes to ring buffer
    let (filled_tx, filled_rx) = unbounded();
    let (empty_tx, _empty_rx) = unbounded();

    let channels = DeviceChannels {
        filled_rx,
        empty_tx,
    };

    let ring_buffer_path = "/tmp/test_audio_source_ringbuf";
    let _ = std::fs::remove_file(ring_buffer_path);
    let ring_buffer = RingBufferWriter::new(ring_buffer_path, 48000, 1, 1).unwrap();
    let ring_buffer_arc = Arc::new(Mutex::new(ring_buffer));

    let test_samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
    let packet = PacketBuffer {
        data: SampleData::F32(test_samples),
        sample_rate: 48000,
        num_channels: 1,
        timestamp: Some(3000000),
    };

    filled_tx.send(packet).unwrap();

    let config = serde_json::json!({
        "sample_rate": 48000
    });

    let mut node = AudioSourceNode::with_device(channels, Some(ring_buffer_arc.clone()));
    node.on_create(config).await.unwrap();

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
async fn test_audio_source_node_silent_writes_to_ring_buffer() {
    // Test that silent audio also writes to ring buffer
    let ring_buffer_path = "/tmp/test_audio_source_silent_ringbuf";
    let _ = std::fs::remove_file(ring_buffer_path);
    let ring_buffer = RingBufferWriter::new(ring_buffer_path, 48000, 1, 1).unwrap();
    let ring_buffer_arc = Arc::new(Mutex::new(ring_buffer));

    let config = serde_json::json!({
        "sample_rate": 48000,
        "buffer_size": 1024
    });

    let mut node = AudioSourceNode::default();
    node.set_ring_buffer(Some(ring_buffer_arc.clone()));
    node.on_create(config).await.unwrap();

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
async fn test_audio_source_node_sequence_increment() {
    // Test that sequence ID increments correctly
    let config = serde_json::json!({
        "sample_rate": 48000,
        "buffer_size": 512
    });

    let mut node = AudioSourceNode::default();
    node.on_create(config).await.unwrap();

    for i in 0..5 {
        let input_frame = DataFrame::new(0, i);
        let output_frame = node.process(input_frame).await.unwrap();
        assert_eq!(output_frame.sequence_id, i + 1);
    }
}

#[tokio::test]
async fn test_audio_source_node_backward_compatibility() {
    // Test that existing code using AudioSourceNode still works
    let config = serde_json::json!({
        "sample_rate": 44100,
        "buffer_size": 2048,
        "num_channels": 1
    });

    let mut node = AudioSourceNode::default();
    node.on_create(config).await.unwrap();

    let input_frame = DataFrame::new(0, 0);
    let output_frame = node.process(input_frame).await.unwrap();

    // Should generate silent audio
    assert!(output_frame.payload.contains_key("main_channel"));
    let main_channel = output_frame.payload.get("main_channel").unwrap();
    assert_eq!(main_channel.len(), 2048);
}
