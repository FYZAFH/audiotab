use crate::core::DataFrame;
use crate::hal::types::{PacketBuffer, SampleData, SampleFormat};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Convert PacketBuffer (native format) to DataFrame (f64)
pub fn packet_to_frame(packet: &PacketBuffer, sequence_id: u64) -> Result<DataFrame> {
    let timestamp = packet.derive_timestamp(sequence_id);

    // Get total samples and samples per channel
    let total_samples = match &packet.data {
        SampleData::I16(v) => v.len(),
        SampleData::I24(v) => v.len() / 3,
        SampleData::I32(v) => v.len(),
        SampleData::F32(v) => v.len(),
        SampleData::F64(v) => v.len(),
        SampleData::U8(v) => v.len(),
        SampleData::Bytes(_) => anyhow::bail!("Cannot convert Bytes to DataFrame"),
    };

    let samples_per_channel = total_samples / packet.num_channels;

    // Convert and de-interleave samples
    let mut payload: HashMap<String, Arc<Vec<f64>>> = HashMap::new();

    for ch in 0..packet.num_channels {
        let mut channel_data = Vec::with_capacity(samples_per_channel);

        for frame in 0..samples_per_channel {
            let index = frame * packet.num_channels + ch;

            let value = match &packet.data {
                SampleData::I16(v) => v[index] as f64 / 32768.0,
                SampleData::I24(v) => {
                    // 24-bit is stored as 3 bytes (little-endian)
                    let byte_index = index * 3;
                    let b0 = v[byte_index] as i32;
                    let b1 = v[byte_index + 1] as i32;
                    let b2 = v[byte_index + 2] as i8 as i32;  // Sign-extend the high byte
                    let sample24 = (b2 << 16) | (b1 << 8) | b0;
                    sample24 as f64 / 8388608.0  // 2^23
                }
                SampleData::I32(v) => v[index] as f64 / 2147483648.0,  // 2^31
                SampleData::F32(v) => v[index] as f64,
                SampleData::F64(v) => v[index],
                SampleData::U8(v) => (v[index] as f64 - 128.0) / 128.0,
                SampleData::Bytes(_) => unreachable!(),
            };

            channel_data.push(value);
        }

        payload.insert(format!("ch{}", ch), Arc::new(channel_data));
    }

    let mut metadata = HashMap::new();
    metadata.insert("sample_rate".to_string(), packet.sample_rate.to_string());

    Ok(DataFrame {
        timestamp,
        sequence_id,
        payload,
        metadata,
    })
}

/// Convert DataFrame (f64) back to PacketBuffer (native format)
pub fn frame_to_packet(frame: &DataFrame, format: SampleFormat, sample_rate: u64) -> Result<PacketBuffer> {
    // Get channels from payload
    let num_channels = frame.payload.len();
    if num_channels == 0 {
        anyhow::bail!("DataFrame has no channels");
    }

    // Get samples per channel (assume all channels have same length)
    let samples_per_channel = frame.payload.values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No channels in DataFrame"))?
        .len();

    // Interleave channels back
    let total_samples = samples_per_channel * num_channels;

    let data = match format {
        SampleFormat::I16 => {
            let mut samples = Vec::with_capacity(total_samples);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    let f64_value = channel_data[frame_idx];
                    let i16_value = (f64_value * 32768.0).clamp(-32768.0, 32767.0) as i16;
                    samples.push(i16_value);
                }
            }
            SampleData::I16(samples)
        }
        SampleFormat::I24 => {
            let mut bytes = Vec::with_capacity(total_samples * 3);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    let f64_value = channel_data[frame_idx];
                    let i24_value = (f64_value * 8388608.0).clamp(-8388608.0, 8388607.0) as i32;

                    // Store as 3 bytes (little-endian)
                    bytes.push((i24_value & 0xFF) as u8);
                    bytes.push(((i24_value >> 8) & 0xFF) as u8);
                    bytes.push(((i24_value >> 16) & 0xFF) as u8);
                }
            }
            SampleData::I24(bytes)
        }
        SampleFormat::I32 => {
            let mut samples = Vec::with_capacity(total_samples);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    let f64_value = channel_data[frame_idx];
                    let i32_value = (f64_value * 2147483648.0).clamp(-2147483648.0, 2147483647.0) as i32;
                    samples.push(i32_value);
                }
            }
            SampleData::I32(samples)
        }
        SampleFormat::F32 => {
            let mut samples = Vec::with_capacity(total_samples);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    samples.push(channel_data[frame_idx] as f32);
                }
            }
            SampleData::F32(samples)
        }
        SampleFormat::F64 => {
            let mut samples = Vec::with_capacity(total_samples);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    samples.push(channel_data[frame_idx]);
                }
            }
            SampleData::F64(samples)
        }
        SampleFormat::U8 => {
            let mut samples = Vec::with_capacity(total_samples);
            for frame_idx in 0..samples_per_channel {
                for ch in 0..num_channels {
                    let channel_data = frame.payload.get(&format!("ch{}", ch))
                        .ok_or_else(|| anyhow::anyhow!("Missing channel ch{}", ch))?;
                    let f64_value = channel_data[frame_idx];
                    let u8_value = ((f64_value * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
                    samples.push(u8_value);
                }
            }
            SampleData::U8(samples)
        }
    };

    Ok(PacketBuffer {
        data,
        sample_rate,
        num_channels,
        timestamp: Some(frame.timestamp),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i16_to_frame_conversion() {
        // Create PacketBuffer with I16 samples
        // Max i16 value is 32767, min is -32768
        // We'll test with known values that normalize well
        let samples = vec![
            0i16,      // 0.0
            16384,     // ~0.5 (16384 / 32768.0 = 0.5)
            -16384,    // ~-0.5
            32767,     // ~1.0 (max)
            -32768,    // -1.0 (min)
        ];

        let packet = PacketBuffer {
            data: SampleData::I16(samples.clone()),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();

        // Should have one channel in payload
        assert_eq!(frame.payload.len(), 1);

        // Get the channel data
        let channel_data = frame.payload.get("ch0").expect("Channel 0 should exist");

        // Verify normalization (I16 divides by 32768.0)
        assert_eq!(channel_data.len(), 5);
        assert!((channel_data[0] - 0.0).abs() < 1e-6);
        assert!((channel_data[1] - 0.5).abs() < 1e-6);
        assert!((channel_data[2] - (-0.5)).abs() < 1e-6);
        assert!((channel_data[3] - (32767.0 / 32768.0)).abs() < 1e-6);
        assert!((channel_data[4] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_f32_to_frame_conversion() {
        // F32 should just cast to f64
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];

        let packet = PacketBuffer {
            data: SampleData::F32(samples.clone()),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();
        let channel_data = frame.payload.get("ch0").unwrap();

        assert_eq!(channel_data.len(), 5);
        assert!((channel_data[0] - 0.0).abs() < 1e-6);
        assert!((channel_data[1] - 0.5).abs() < 1e-6);
        assert!((channel_data[2] - (-0.5)).abs() < 1e-6);
        assert!((channel_data[3] - 1.0).abs() < 1e-6);
        assert!((channel_data[4] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_multi_channel_i16_to_frame() {
        // Interleaved stereo: [L0, R0, L1, R1, L2, R2]
        let samples = vec![
            1000i16, 2000i16,  // Frame 0: L=1000, R=2000
            3000i16, 4000i16,  // Frame 1: L=3000, R=4000
            5000i16, 6000i16,  // Frame 2: L=5000, R=6000
        ];

        let packet = PacketBuffer {
            data: SampleData::I16(samples),
            sample_rate: 48000,
            num_channels: 2,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();

        // Should have two channels
        assert_eq!(frame.payload.len(), 2);

        let ch0 = frame.payload.get("ch0").unwrap();
        let ch1 = frame.payload.get("ch1").unwrap();

        // Each channel should have 3 samples (de-interleaved)
        assert_eq!(ch0.len(), 3);
        assert_eq!(ch1.len(), 3);

        // Verify de-interleaving
        assert!((ch0[0] - (1000.0 / 32768.0)).abs() < 1e-6);
        assert!((ch0[1] - (3000.0 / 32768.0)).abs() < 1e-6);
        assert!((ch0[2] - (5000.0 / 32768.0)).abs() < 1e-6);

        assert!((ch1[0] - (2000.0 / 32768.0)).abs() < 1e-6);
        assert!((ch1[1] - (4000.0 / 32768.0)).abs() < 1e-6);
        assert!((ch1[2] - (6000.0 / 32768.0)).abs() < 1e-6);
    }

    #[test]
    fn test_frame_to_i16_round_trip() {
        // Create original packet
        let original_samples = vec![0i16, 16384, -16384, 32767, -32768];
        let original_packet = PacketBuffer {
            data: SampleData::I16(original_samples.clone()),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        // Convert to frame
        let frame = packet_to_frame(&original_packet, 1).unwrap();

        // Convert back to packet
        let reconstructed = frame_to_packet(&frame, SampleFormat::I16, 48000).unwrap();

        // Verify round-trip
        match reconstructed.data {
            SampleData::I16(samples) => {
                assert_eq!(samples.len(), original_samples.len());
                for (i, &sample) in samples.iter().enumerate() {
                    // Allow for minor rounding errors
                    assert!((sample - original_samples[i]).abs() <= 1);
                }
            }
            _ => panic!("Expected I16 data"),
        }
    }

    #[test]
    fn test_i32_conversion() {
        let samples = vec![
            0i32,
            1073741824,    // ~0.5
            -1073741824,   // ~-0.5
            2147483647,    // max ~1.0
            -2147483648,   // min -1.0
        ];

        let packet = PacketBuffer {
            data: SampleData::I32(samples),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();
        let channel_data = frame.payload.get("ch0").unwrap();

        // Verify normalization (I32 divides by 2^31 = 2147483648.0)
        assert!((channel_data[0] - 0.0).abs() < 1e-6);
        assert!((channel_data[1] - 0.5).abs() < 1e-6);
        assert!((channel_data[2] - (-0.5)).abs() < 1e-6);
        assert!((channel_data[3] - (2147483647.0 / 2147483648.0)).abs() < 1e-6);
        assert!((channel_data[4] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_u8_conversion() {
        // U8: 0 = -1.0, 128 = 0.0, 255 = ~1.0
        let samples = vec![128u8, 192, 64, 255, 0];

        let packet = PacketBuffer {
            data: SampleData::U8(samples),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();
        let channel_data = frame.payload.get("ch0").unwrap();

        // Verify normalization: (value - 128) / 128.0
        assert!((channel_data[0] - 0.0).abs() < 1e-6);       // 128 -> 0.0
        assert!((channel_data[1] - 0.5).abs() < 1e-6);       // 192 -> 0.5
        assert!((channel_data[2] - (-0.5)).abs() < 1e-6);    // 64 -> -0.5
        assert!((channel_data[3] - (127.0 / 128.0)).abs() < 1e-6);  // 255 -> ~0.99
        assert!((channel_data[4] - (-1.0)).abs() < 1e-6);    // 0 -> -1.0
    }

    #[test]
    fn test_f64_conversion() {
        // F64 should be pass-through
        let samples = vec![0.0f64, 0.5, -0.5, 1.0, -1.0];

        let packet = PacketBuffer {
            data: SampleData::F64(samples.clone()),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };

        let frame = packet_to_frame(&packet, 1).unwrap();
        let channel_data = frame.payload.get("ch0").unwrap();

        // Should be exact (no conversion needed)
        for (i, &expected) in samples.iter().enumerate() {
            assert!((channel_data[i] - expected).abs() < 1e-15);
        }
    }

    #[test]
    fn test_all_formats_round_trip() {
        // Test I16
        let i16_packet = PacketBuffer {
            data: SampleData::I16(vec![0, 16384, -16384]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };
        let frame = packet_to_frame(&i16_packet, 1).unwrap();
        let _ = frame_to_packet(&frame, SampleFormat::I16, 48000).unwrap();

        // Test I32
        let i32_packet = PacketBuffer {
            data: SampleData::I32(vec![0, 1073741824, -1073741824]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };
        let frame = packet_to_frame(&i32_packet, 1).unwrap();
        let _ = frame_to_packet(&frame, SampleFormat::I32, 48000).unwrap();

        // Test F32
        let f32_packet = PacketBuffer {
            data: SampleData::F32(vec![0.0, 0.5, -0.5]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };
        let frame = packet_to_frame(&f32_packet, 1).unwrap();
        let _ = frame_to_packet(&frame, SampleFormat::F32, 48000).unwrap();

        // Test F64
        let f64_packet = PacketBuffer {
            data: SampleData::F64(vec![0.0, 0.5, -0.5]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };
        let frame = packet_to_frame(&f64_packet, 1).unwrap();
        let _ = frame_to_packet(&frame, SampleFormat::F64, 48000).unwrap();

        // Test U8
        let u8_packet = PacketBuffer {
            data: SampleData::U8(vec![128, 192, 64]),
            sample_rate: 48000,
            num_channels: 1,
            timestamp: Some(1000000),
        };
        let frame = packet_to_frame(&u8_packet, 1).unwrap();
        let _ = frame_to_packet(&frame, SampleFormat::U8, 48000).unwrap();
    }
}
