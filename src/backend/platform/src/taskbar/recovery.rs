use std::time::{Duration, Instant};

/// Retry failed desktop integrations without rebuilding them at the sampling rate.
/// A changed configuration is an explicit new attempt; disabling cancels retries.
pub struct RetrySchedule {
    revision: Option<u64>,
    failures: u32,
    next_attempt: Instant,
}

impl RetrySchedule {
    pub fn new(now: Instant) -> Self {
        Self {
            revision: None,
            failures: 0,
            next_attempt: now,
        }
    }

    pub fn configure(&mut self, enabled: bool, revision: u64, now: Instant) -> bool {
        let revision = enabled.then_some(revision);
        if self.revision == revision {
            return false;
        }
        self.revision = revision;
        self.succeeded(now);
        true
    }

    pub fn due(&self, now: Instant) -> bool {
        self.revision.is_some() && now >= self.next_attempt
    }

    pub fn failed(&mut self, now: Instant) {
        if self.revision.is_none() {
            return;
        }
        self.failures = (self.failures + 1).min(6);
        self.next_attempt = now + Duration::from_secs((1 << (self.failures - 1)).min(30));
    }

    pub fn succeeded(&mut self, now: Instant) {
        self.failures = 0;
        self.next_attempt = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_configuration_retries_with_bounded_exponential_delays() {
        let mut now = Instant::now();
        let mut retry = RetrySchedule::new(now);
        retry.configure(true, 1, now);
        assert!(retry.due(now));
        for seconds in [1, 2, 4, 8, 16, 30, 30, 30] {
            retry.failed(now);
            assert!(!retry.configure(true, 1, now));
            assert!(!retry.due(now + Duration::from_secs(seconds) - Duration::from_millis(1)));
            now += Duration::from_secs(seconds);
            assert!(retry.due(now));
        }
    }

    #[test]
    fn configuration_changes_retry_immediately_and_success_resets_failures() {
        let now = Instant::now();
        let mut retry = RetrySchedule::new(now);
        retry.configure(true, 1, now);
        for _ in 0..8 {
            retry.failed(now);
        }
        assert!(!retry.due(now));
        assert!(retry.configure(true, 2, now));
        assert!(retry.due(now));
        retry.failed(now);
        assert!(retry.due(now + Duration::from_secs(1)));
        retry.failed(now);
        retry.succeeded(now);
        retry.failed(now);
        assert!(retry.due(now + Duration::from_secs(1)));
    }

    #[test]
    fn disabling_cancels_retries_and_reenabling_starts_immediately() {
        let now = Instant::now();
        let mut retry = RetrySchedule::new(now);
        retry.configure(true, 1, now);
        retry.failed(now);
        retry.configure(false, 2, now);
        retry.failed(now);
        assert!(!retry.due(now + Duration::from_secs(3600)));
        retry.configure(true, 3, now);
        assert!(retry.due(now));
    }
}
