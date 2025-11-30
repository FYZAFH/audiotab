pub mod traits;
pub mod types;
pub mod registry;
pub mod drivers;
pub mod channel_mapper;
pub mod device_profile;
pub mod device_storage;
pub mod device_manager;
pub mod registered;
pub mod format_converter;

pub use traits::{HardwareDriver, Device};
pub use types::{
    HardwareType, DeviceInfo, DeviceConfig, DeviceCapabilities,
    DeviceChannels, PacketBuffer, SampleData, SampleFormat,
    ChannelMapping, ChannelRoute, Calibration,
};
pub use registry::HardwareRegistry;
pub use drivers::AudioDriver;
pub use channel_mapper::ChannelMapper;
pub use device_profile::{DeviceProfile, DeviceMetadata};
pub use device_storage::DeviceStorage;
pub use device_manager::DeviceManager;
pub use registered::*;
