use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// Hardware classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareType {
    /// Full framework support - time-series samples
    Acoustic,
    /// Developer-defined usage
    Special,
}

/// Device discovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub hardware_type: HardwareType,
    pub driver_id: String,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceConfig {
    pub name: String,
    pub sample_rate: u64,
    pub format: SampleFormat,
    pub buffer_size: usize,
    pub channel_mapping: ChannelMapping,
    pub calibration: Calibration,
}

/// Sample data format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    I16,  // 16-bit PCM
    I24,  // 24-bit
    I32,  // 32-bit integer
    F32,  // 32-bit float
    F64,  // 64-bit float
    U8,   // 8-bit unsigned
}

impl Default for SampleFormat {
    fn default() -> Self {
        SampleFormat::F32
    }
}

/// Channel mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelMapping {
    pub physical_channels: usize,
    pub virtual_channels: usize,
    pub routing: Vec<ChannelRoute>,
}

impl Default for ChannelMapping {
    fn default() -> Self {
        Self {
            physical_channels: 0,
            virtual_channels: 0,
            routing: Vec::new(),
        }
    }
}

/// Channel routing rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelRoute {
    Direct(usize),          // Phys[i] -> Virt[i]
    Reorder(Vec<usize>),    // Phys[1,2,3] -> Virt[3,2,1]
    Merge(Vec<usize>),      // Phys[1,2,3] -> Virt[1]
    Duplicate(usize),       // Phys[1] -> Virt[1,2,3]
}

/// Calibration settings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Calibration {
    pub gain: f64,    // Multiply for voltage
    pub offset: f64,  // Add for SPL
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            gain: 1.0,
            offset: 0.0,
        }
    }
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub can_input: bool,
    pub can_output: bool,
    pub supported_formats: Vec<SampleFormat>,
    pub supported_sample_rates: Vec<u64>,
    pub max_channels: usize,
}

/// Channels for buffer ping-pong pattern
#[derive(Clone)]
pub struct DeviceChannels {
    /// Receive filled buffers from hardware
    pub filled_rx: Receiver<PacketBuffer>,
    /// Send empty buffers back to hardware
    pub empty_tx: Sender<PacketBuffer>,
}

/// Packet buffer for streaming data
#[derive(Debug, Clone)]
pub struct PacketBuffer {
    pub data: SampleData,
    pub sample_rate: u64,
    pub num_channels: usize,
    pub timestamp: Option<u64>,  // Nanoseconds
}

/// Sample data in native format
#[derive(Debug, Clone)]
pub enum SampleData {
    I16(Vec<i16>),
    I24(Vec<u8>),  // 3 bytes per sample
    I32(Vec<i32>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    U8(Vec<u8>),
    Bytes(Vec<u8>),  // For special hardware
}

impl PacketBuffer {
    pub fn new(format: SampleFormat, buffer_size: usize, num_channels: usize) -> Self {
        let capacity = buffer_size * num_channels;
        let data = match format {
            SampleFormat::I16 => SampleData::I16(vec![0i16; capacity]),
            SampleFormat::I24 => SampleData::I24(vec![0u8; capacity * 3]),
            SampleFormat::I32 => SampleData::I32(vec![0i32; capacity]),
            SampleFormat::F32 => SampleData::F32(vec![0.0f32; capacity]),
            SampleFormat::F64 => SampleData::F64(vec![0.0f64; capacity]),
            SampleFormat::U8 => SampleData::U8(vec![0u8; capacity]),
        };

        Self {
            data,
            sample_rate: 48000,  // Default
            num_channels,
            timestamp: None,
        }
    }

    /// Derive timestamp from packet index if not provided
    pub fn derive_timestamp(&self, packet_index: u64) -> u64 {
        if let Some(ts) = self.timestamp {
            return ts;
        }

        let samples_per_packet = match &self.data {
            SampleData::I16(v) => v.len() / self.num_channels,
            SampleData::I32(v) => v.len() / self.num_channels,
            SampleData::F32(v) => v.len() / self.num_channels,
            SampleData::F64(v) => v.len() / self.num_channels,
            SampleData::U8(v) => v.len() / self.num_channels,
            SampleData::I24(v) => (v.len() / 3) / self.num_channels,
            SampleData::Bytes(_) => 0,
        };

        let samples_elapsed = packet_index * samples_per_packet as u64;
        (samples_elapsed * 1_000_000_000) / self.sample_rate
    }
}
