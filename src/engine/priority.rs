use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Task priority levels with target latencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// 0-10ms: Real-time monitoring, safety-critical
    Critical,
    /// 10-50ms: User-triggered interactive analysis
    High,
    /// 50-200ms: Background automated testing
    Normal,
    /// >200ms: Batch processing, exports
    Low,
}

impl Priority {
    /// Get target latency in milliseconds
    pub fn target_latency_ms(&self) -> u64 {
        match self {
            Priority::Critical => 10,
            Priority::High => 50,
            Priority::Normal => 200,
            Priority::Low => 1000,
        }
    }

    /// Get numeric value for comparison (higher = more urgent)
    pub fn value(&self) -> u8 {
        match self {
            Priority::Critical => 3,
            Priority::High => 2,
            Priority::Normal => 1,
            Priority::Low => 0,
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(&other.value())
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_target_latency() {
        assert_eq!(Priority::Critical.target_latency_ms(), 10);
        assert_eq!(Priority::Low.target_latency_ms(), 1000);
    }
}
