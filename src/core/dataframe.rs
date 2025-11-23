use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Basic data unit passed between processing nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFrame {
    /// Timestamp in microseconds since epoch
    pub timestamp: u64,

    /// Sequential frame number for ordering
    pub sequence_id: u64,

    /// Multi-channel data keyed by channel name
    pub payload: HashMap<String, Vec<f64>>,

    /// Side-channel information (gain, sample_rate, etc)
    pub metadata: HashMap<String, String>,
}

impl DataFrame {
    pub fn new(timestamp: u64, sequence_id: u64) -> Self {
        Self {
            timestamp,
            sequence_id,
            payload: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}
