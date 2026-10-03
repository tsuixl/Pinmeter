use crate::domain::{Frame, Reading, Status};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_EVENTS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRule {
    pub enabled: bool,
    pub threshold_percent: u8,
    pub duration_seconds: u16,
    pub cooldown_seconds: u32,
}
impl Default for AlertRule {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_percent: 90,
            duration_seconds: 30,
            cooldown_seconds: 300,
        }
    }
}
impl AlertRule {
    fn validate(&self) -> Result<(), String> {
        if !(1..=100).contains(&self.threshold_percent)
            || !(5..=600).contains(&self.duration_seconds)
            || !(60..=86400).contains(&self.cooldown_seconds)
        {
            return Err("提醒阈值须为 1–100%，持续 5–600 秒，冷却 60–86400 秒".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_minute: u16,
    pub end_minute: u16,
}
impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start_minute: 22 * 60,
            end_minute: 8 * 60,
        }
    }
}
impl QuietHours {
    pub fn contains(&self, local_minute: Option<u16>) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(minute) = local_minute.filter(|minute| *minute < 1440) else {
            // Unknown local time must not break a user's explicit quiet preference.
            return true;
        };
        if self.start_minute < self.end_minute {
            minute >= self.start_minute && minute < self.end_minute
        } else {
            minute >= self.start_minute || minute < self.end_minute
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertsConfig {
    pub cpu: AlertRule,
    pub memory: AlertRule,
    pub quiet: QuietHours,
}
impl AlertsConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.cpu.validate()?;
        self.memory.validate()?;
        if self.quiet.start_minute >= 1440
            || self.quiet.end_minute >= 1440
            || self.quiet.start_minute == self.quiet.end_minute
        {
            return Err("静默起止时间须有效且不能相同".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertMetric {
    Cpu,
    Memory,
}
impl AlertMetric {
    pub fn key(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AlertEvent {
    pub id: u64,
    pub metric: AlertMetric,
    pub at_ms: u64,
    pub value: f64,
    pub threshold_percent: u8,
    pub duration_seconds: u16,
    pub acknowledged: bool,
}

#[derive(Default)]
struct RuleState {
    since: Option<u64>,
    last_fired: Option<u64>,
    emitted_in_episode: bool,
}
impl RuleState {
    fn reset_episode(&mut self) {
        self.since = None;
        self.emitted_in_episode = false;
    }
    fn accept(&mut self, rule: &AlertRule, reading: &Reading, mono: u64) -> Option<f64> {
        let value = reading.value.filter(|value| {
            reading.status == Status::Normal && value.is_finite() && (0.0..=100.0).contains(value)
        });
        if !rule.enabled || value.is_none_or(|value| value < f64::from(rule.threshold_percent)) {
            self.reset_episode();
            return None;
        }
        let since = *self.since.get_or_insert(mono);
        let cooling_down = self.last_fired.is_some_and(|last| {
            mono.saturating_sub(last) < u64::from(rule.cooldown_seconds) * 1000
        });
        if !self.emitted_in_episode
            && !cooling_down
            && mono.saturating_sub(since) >= u64::from(rule.duration_seconds) * 1000
        {
            self.last_fired = Some(mono);
            self.emitted_in_episode = true;
            value
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct AlertEngine {
    config: AlertsConfig,
    cpu: RuleState,
    memory: RuleState,
    previous: Option<(u64, u64, u64)>, // cursor, monotonic time, generation
    events: VecDeque<AlertEvent>,
    next_id: u64,
    revision: u64,
}
impl AlertEngine {
    pub fn events(&self) -> &VecDeque<AlertEvent> {
        &self.events
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn unread_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| !event.acknowledged)
            .count()
    }
    pub fn acknowledge(&mut self, id: u64) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id)
            && !event.acknowledged
        {
            event.acknowledged = true;
            self.revision = self.revision.saturating_add(1);
        }
    }
    pub fn clear_through(&mut self, id: u64) {
        let count = self.events.len();
        self.events.retain(|event| event.id > id);
        if count != self.events.len() {
            self.revision = self.revision.saturating_add(1);
        }
    }
    pub fn accept(
        &mut self,
        config: &AlertsConfig,
        frame: &Frame,
        local_minute: Option<u16>,
        max_gap_ms: u64,
    ) {
        if self.config != *config {
            self.cpu.reset_episode();
            self.memory.reset_episode();
            self.config = config.clone();
        }
        if let Some((cursor, mono, generation)) = self.previous {
            if frame.cursor <= cursor || frame.elapsed_ms <= mono {
                return;
            }
            if frame.elapsed_ms - mono > max_gap_ms || generation != frame.generation {
                self.cpu.reset_episode();
                self.memory.reset_episode();
            }
        }
        self.previous = Some((frame.cursor, frame.elapsed_ms, frame.generation));
        if config.validate().is_err() || config.quiet.contains(local_minute) {
            self.cpu.reset_episode();
            self.memory.reset_episode();
            return;
        }
        let cpu = self.cpu.accept(&config.cpu, &frame.cpu, frame.elapsed_ms);
        let memory = self
            .memory
            .accept(&config.memory, &frame.memory, frame.elapsed_ms);
        for (metric, value, rule) in [
            (AlertMetric::Cpu, cpu, &config.cpu),
            (AlertMetric::Memory, memory, &config.memory),
        ] {
            if let Some(value) = value {
                self.next_id = self.next_id.saturating_add(1);
                self.events.push_front(AlertEvent {
                    id: self.next_id,
                    metric,
                    at_ms: frame.at_ms,
                    value,
                    threshold_percent: rule.threshold_percent,
                    duration_seconds: rule.duration_seconds,
                    acknowledged: false,
                });
                self.events.truncate(MAX_EVENTS);
                self.revision = self.revision.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(value: Option<f64>, status: Status) -> Reading {
        Reading {
            value,
            status,
            valid_at_ms: None,
            valid_mono_ms: None,
            source: "test",
            semantic: "test",
            detail: String::new(),
        }
    }
    fn frame(mono: u64, value: f64) -> Frame {
        Frame {
            gpus: vec![],
            cursor: mono + 1,
            at_ms: 100_000 + mono,
            elapsed_ms: mono,
            generation: 0,
            network_generation: 0,
            network_id: None,
            cpu: reading(Some(value), Status::Normal),
            cpu_temperature: reading(None, Status::Unsupported),
            memory: reading(Some(95.0), Status::Normal),
            download: reading(None, Status::Warming),
            upload: reading(None, Status::Warming),
            memory_used: None,
            memory_total: None,
        }
    }
    fn config() -> AlertsConfig {
        let mut config = AlertsConfig::default();
        config.cpu.enabled = true;
        config.cpu.duration_seconds = 5;
        config.cpu.cooldown_seconds = 60;
        config
    }
    fn feed(engine: &mut AlertEngine, config: &AlertsConfig, start: u64, end: u64, value: f64) {
        for mono in (start..=end).step_by(1000) {
            engine.accept(config, &frame(mono, value), Some(12 * 60), 3000);
        }
    }

    #[test]
    fn defaults_are_off_and_configuration_is_bounded() {
        let mut engine = AlertEngine::default();
        feed(&mut engine, &AlertsConfig::default(), 0, 40000, 100.0);
        assert!(engine.events().is_empty());
        let mut c = config();
        c.cpu.threshold_percent = 0;
        assert!(c.validate().is_err());
        c = config();
        c.cpu.duration_seconds = 4;
        assert!(c.validate().is_err());
        c = config();
        c.memory.cooldown_seconds = 86401;
        assert!(c.validate().is_err());
        c = config();
        c.quiet.start_minute = c.quiet.end_minute;
        assert!(c.validate().is_err());
    }

    #[test]
    fn sustained_boundary_and_one_event_per_episode() {
        let mut e = AlertEngine::default();
        let c = config();
        feed(&mut e, &c, 0, 4000, 90.0);
        assert!(e.events().is_empty());
        feed(&mut e, &c, 5000, 90000, 90.0);
        assert_eq!(e.events().len(), 1);
        assert_eq!(e.events()[0].at_ms, 105000);
        assert_eq!(e.events()[0].metric, AlertMetric::Cpu);
    }

    #[test]
    fn recovery_requires_duration_and_cooldown_then_can_fire_again() {
        let mut e = AlertEngine::default();
        let c = config();
        feed(&mut e, &c, 0, 5000, 95.0);
        feed(&mut e, &c, 6000, 6000, 89.0);
        feed(&mut e, &c, 7000, 64000, 95.0);
        assert_eq!(e.events().len(), 1);
        feed(&mut e, &c, 65000, 65000, 95.0);
        assert_eq!(e.events().len(), 2);
        let id = e.events()[0].id;
        e.clear_through(id);
        feed(&mut e, &c, 66000, 66000, 80.0);
        feed(&mut e, &c, 67000, 73000, 95.0);
        assert!(
            e.events().is_empty(),
            "clearing history must retain cooldown"
        );
    }

    #[test]
    fn invalid_sample_gap_and_generation_restart_duration() {
        for status in [
            Status::Warming,
            Status::Failed,
            Status::Stale,
            Status::PermissionDenied,
            Status::Unsupported,
        ] {
            let mut e = AlertEngine::default();
            let c = config();
            feed(&mut e, &c, 0, 4000, 95.0);
            let mut invalid = frame(5000, 95.0);
            invalid.cpu.status = status;
            e.accept(&c, &invalid, Some(720), 3000);
            feed(&mut e, &c, 6000, 10000, 95.0);
            assert!(e.events().is_empty());
            feed(&mut e, &c, 11000, 11000, 95.0);
            assert_eq!(e.events().len(), 1);
        }
        for invalid_value in [f64::NAN, f64::INFINITY, -1.0, 100.1] {
            let mut e = AlertEngine::default();
            feed(&mut e, &config(), 0, 10000, invalid_value);
            assert!(e.events().is_empty());
        }
        let mut e = AlertEngine::default();
        feed(&mut e, &config(), 0, 4000, 95.0);
        feed(&mut e, &config(), 100000, 104000, 95.0);
        assert!(e.events().is_empty());
        let mut changed = frame(105000, 95.0);
        changed.generation = 1;
        e.accept(&config(), &changed, Some(720), 3000);
        assert!(e.events().is_empty());
    }

    #[test]
    fn duplicates_and_backwards_samples_do_not_fake_elapsed_time() {
        let mut e = AlertEngine::default();
        let c = config();
        feed(&mut e, &c, 0, 4000, 95.0);
        e.accept(&c, &frame(4000, 95.0), Some(720), 3000);
        e.accept(&c, &frame(1000, 95.0), Some(720), 3000);
        let mut repeated = frame(10000, 95.0);
        repeated.cursor = 4001;
        e.accept(&c, &repeated, Some(720), 3000);
        assert!(e.events().is_empty());
    }

    #[test]
    fn quiet_hours_cross_midnight_and_never_queue_a_catchup() {
        let mut c = config();
        c.quiet.enabled = true;
        for minute in [Some(22 * 60), Some(0), Some(8 * 60 - 1), None] {
            assert!(c.quiet.contains(minute));
        }
        assert!(!c.quiet.contains(Some(8 * 60)));
        let mut e = AlertEngine::default();
        feed(&mut e, &c, 0, 4000, 95.0);
        for mono in (5000..=12000).step_by(1000) {
            e.accept(&c, &frame(mono, 95.0), Some(23 * 60), 3000);
        }
        feed(&mut e, &c, 13000, 17000, 95.0);
        assert!(e.events().is_empty());
        feed(&mut e, &c, 18000, 18000, 95.0);
        assert_eq!(e.events().len(), 1);
        c.quiet.start_minute = 9 * 60;
        c.quiet.end_minute = 18 * 60;
        assert!(c.quiet.contains(Some(9 * 60)));
        assert!(!c.quiet.contains(Some(18 * 60)));
    }

    #[test]
    fn bounded_history_and_acknowledgement_keep_newer_events() {
        let mut e = AlertEngine::default();
        let mut c = config();
        c.memory.enabled = true;
        c.memory.duration_seconds = 5;
        c.memory.cooldown_seconds = 60;
        for iteration in 0..25 {
            let start = iteration * 70000;
            feed(&mut e, &c, start, start, 0.0);
            feed(&mut e, &c, start + 1000, start + 6000, 95.0);
        }
        assert_eq!(e.events().len(), MAX_EVENTS);
        let id = e.events()[1].id;
        let newest = e.events()[0].id;
        e.acknowledge(newest);
        assert_eq!(e.unread_count(), MAX_EVENTS - 1);
        e.clear_through(id);
        assert_eq!(e.events().len(), 1);
        assert_eq!(e.events()[0].id, newest);
    }
}
