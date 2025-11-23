use crate::core::DataFrame;

#[derive(Debug, Clone)]
pub enum ErrorPolicy {
    /// Propagate error up (current behavior - stops pipeline)
    Propagate,

    /// Skip the errored frame and continue processing
    SkipFrame,

    /// Use a default/empty frame when error occurs
    UseDefault(DataFrame),
}

#[derive(Debug, Clone)]
pub enum RestartStrategy {
    /// Never restart node after error
    Never,

    /// Restart immediately on error
    Immediate,

    /// Exponential backoff restart
    Exponential {
        base_ms: u64,
        max_ms: u64,
        max_attempts: usize,
    },

    /// Circuit breaker pattern
    CircuitBreaker {
        error_threshold: usize,
        timeout_ms: u64,
    },
}
