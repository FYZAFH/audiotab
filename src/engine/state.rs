use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Pipeline execution states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    Initializing { progress: u8 }, // 0-100
    Running {
        #[serde(skip)]
        start_time: Option<Instant>,
        frames_processed: u64,
    },
    Paused {
        #[serde(skip)]
        pause_time: Option<Instant>,
    },
    Completed {
        #[serde(skip)]
        duration: Option<Duration>,
        total_frames: u64,
    },
    Error {
        error_msg: String,
        recoverable: bool,
    },
}

impl PipelineState {
    /// Check if transition from current state to target state is valid
    pub fn can_transition_to(&self, target: &PipelineState) -> bool {
        use PipelineState::*;

        matches!(
            (self, target),
            // From Idle
            (Idle, Initializing { .. }) |

            // From Initializing
            (Initializing { .. }, Running { .. }) |
            (Initializing { .. }, Error { .. }) |

            // From Running
            (Running { .. }, Paused { .. }) |
            (Running { .. }, Completed { .. }) |
            (Running { .. }, Error { .. }) |

            // From Paused
            (Paused { .. }, Running { .. }) |
            (Paused { .. }, Completed { .. }) |
            (Paused { .. }, Error { .. }) |

            // From Completed
            (Completed { .. }, Idle) |

            // From Error
            (Error { recoverable: true, .. }, Idle)
        )
    }

    /// Get human-readable state name
    pub fn name(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Initializing { .. } => "Initializing",
            Self::Running { .. } => "Running",
            Self::Paused { .. } => "Paused",
            Self::Completed { .. } => "Completed",
            Self::Error { .. } => "Error",
        }
    }
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let idle = PipelineState::Idle;
        let init = PipelineState::Initializing { progress: 50 };

        assert!(idle.can_transition_to(&init));
        assert!(!init.can_transition_to(&idle));
    }

    #[test]
    fn test_running_to_paused() {
        let running = PipelineState::Running {
            start_time: None,
            frames_processed: 100,
        };
        let paused = PipelineState::Paused { pause_time: None };

        assert!(running.can_transition_to(&paused));
    }

    #[test]
    fn test_error_recovery() {
        let recoverable_error = PipelineState::Error {
            error_msg: "timeout".to_string(),
            recoverable: true,
        };
        let unrecoverable_error = PipelineState::Error {
            error_msg: "fatal".to_string(),
            recoverable: false,
        };

        assert!(recoverable_error.can_transition_to(&PipelineState::Idle));
        assert!(!unrecoverable_error.can_transition_to(&PipelineState::Idle));
    }
}
