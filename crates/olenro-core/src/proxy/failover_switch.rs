//! Failover switch
//!
//! Manages automatic failover between providers

use crate::error::AppResult;
use crate::proxy::FailoverQueueItem;
use std::collections::VecDeque;

/// Failover switch for managing provider failover
pub struct FailoverSwitch {
    queue: VecDeque<FailoverQueueItem>,
}

impl FailoverSwitch {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Add a provider to the failover queue
    pub fn add(&mut self, item: FailoverQueueItem) {
        self.queue.push_back(item);
    }

    /// Get the next provider to failover to
    pub fn next(&mut self) -> Option<String> {
        self.queue.pop_front().map(|item| item.provider_id)
    }

    /// Check if there are providers in the queue
    pub fn has_next(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for FailoverSwitch {
    fn default() -> Self {
        Self::new()
    }
}
