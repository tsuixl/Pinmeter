use crate::domain::{RawSample, Settings};

pub trait MetricProvider {
    fn sample(&mut self) -> RawSample;
    fn reset_baseline(&mut self);
}

pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Result<Option<Settings>, String>;
    fn save(&self, settings: &Settings) -> Result<(), String>;
}

pub trait Clock {
    fn monotonic_ms(&self) -> u64;
    fn wall_ms(&self) -> u64;
}
