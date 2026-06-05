//! Circuit breaker
//!
//! Circuit breaker implementation for provider failover

use std::time::{Duration, Instant};
use crate::proxy::CircuitBreakerState;

/// Circuit breaker for a single provider
pub struct CircuitBreaker {
    /// Provider ID this breaker is for
    provider_id: String,
    /// Current state
    state: CircuitBreakerState,
    /// Failure count in the current window
    failure_count: u32,
    /// Window start time
    window_start: Instant,
    /// Last failure time
    last_failure: Option<Instant>,
    /// Threshold to open circuit
    threshold: u32,
    /// Timeout before trying half-open
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            window_start: Instant::now(),
            last_failure: None,
            threshold: 5,
            timeout: Duration::from_secs(30),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitBreakerState::Closed;
    }

    /// Record a failed request
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.threshold {
            self.state = CircuitBreakerState::Open;
        }
    }

    /// Check if requests can proceed
    pub fn can_request(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if self.last_failure.map(|t| t.elapsed() >= self.timeout).unwrap_or(false) {
                    self.state = CircuitBreakerState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitBreakerState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new("test".to_string());
        assert!(cb.can_request());

        for _ in 0..5 {
            cb.record_failure();
        }

        assert!(!cb.can_request());
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }
}
