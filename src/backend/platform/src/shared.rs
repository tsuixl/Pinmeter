use pinmeter_core::{
    domain::{Failure, Observation},
    ports::Clock,
};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static MONOTONIC_ORIGIN: OnceLock<Instant> = OnceLock::new();

pub struct SystemClock {
    started: Instant,
}
impl Default for SystemClock {
    fn default() -> Self {
        Self {
            started: *MONOTONIC_ORIGIN.get_or_init(Instant::now),
        }
    }
}
impl Clock for SystemClock {
    fn monotonic_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
    fn wall_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
impl SystemClock {
    pub fn observation<T>(
        &self,
        result: Result<T, Failure>,
        source: &'static str,
        semantic: &'static str,
    ) -> Observation<T> {
        Observation {
            result,
            source,
            semantic,
            mono_ms: self.monotonic_ms(),
            wall_ms: self.wall_ms(),
        }
    }
}
