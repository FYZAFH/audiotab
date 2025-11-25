pub mod traits;
pub mod types;
pub mod registry;

pub use traits::{HardwareDriver, Device};
pub use types::{
    HardwareType, DeviceInfo, DeviceConfig, DeviceCapabilities,
    DeviceChannels, PacketBuffer, SampleData, SampleFormat,
    ChannelMapping, ChannelRoute, Calibration,
};
pub use registry::HardwareRegistry;
