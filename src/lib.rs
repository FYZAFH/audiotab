pub mod buffers;
pub mod core;
pub mod engine;
pub mod hal;
pub mod nodes;
pub mod observability;
pub mod registry;
pub mod resilience;

pub use core::{ProcessingNode, NodeContext, DataFrame};
pub use registry::{NodeMetadata, PortMetadata, ParameterSchema, NodeMetadataFactory};
