pub mod traits;
pub mod types;
pub mod registry;
pub mod drivers;
pub mod channel_mapper;

pub use traits::{HardwareDriver, Device};
pub use types::{
    HardwareType, DeviceInfo, DeviceConfig, DeviceCapabilities,
    DeviceChannels, PacketBuffer, SampleData, SampleFormat,
    ChannelMapping, ChannelRoute, Calibration,
};
pub use registry::HardwareRegistry;
pub use drivers::AudioDriver;
pub use channel_mapper::ChannelMapper;
