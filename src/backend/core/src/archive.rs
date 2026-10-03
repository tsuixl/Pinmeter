use crate::domain::TrafficDelta;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MINUTE: u64 = 60_000;
pub const DAY: u64 = 24 * 60 * MINUTE;
pub const MAX_BUCKETS: usize = 1441;

#[derive(Clone, Debug)]
pub struct ArchiveInput {
    pub wall_ms: u64,
    pub elapsed_ms: u64,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub traffic: Option<TrafficDelta>,
    pub network: Option<NetworkSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkSource {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Aggregate {
    pub weighted_sum: f64,
    pub covered_ms: u64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
impl Aggregate {
    pub fn average(&self) -> Option<f64> {
        (self.covered_ms > 0).then(|| self.weighted_sum / self.covered_ms as f64)
    }
    fn add(&mut self, value: f64, ms: u64) {
        if !value.is_finite() || !(0.0..=100.).contains(&value) {
            return;
        }
        let ms = ms.min(MINUTE.saturating_sub(self.covered_ms));
        if ms == 0 {
            return;
        }
        self.weighted_sum += value * ms as f64;
        self.covered_ms += ms;
        self.min = Some(self.min.map_or(value, |old| old.min(value)));
        self.max = Some(self.max.map_or(value, |old| old.max(value)));
    }
    fn valid(&self) -> bool {
        self.covered_ms <= MINUTE
            && self.weighted_sum.is_finite()
            && self.weighted_sum >= 0.
            && self.weighted_sum <= self.covered_ms as f64 * 100.
            && self
                .min
                .into_iter()
                .chain(self.max)
                .all(|v| v.is_finite() && (0.0..=100.).contains(&v))
            && if self.covered_ms == 0 {
                self.min.is_none() && self.max.is_none()
            } else {
                self.min.zip(self.max).is_some_and(|(min, max)| min <= max)
            }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MinuteBucket {
    pub cpu: Aggregate,
    pub memory: Aggregate,
    pub received: u64,
    pub transmitted: u64,
    pub network_ms: u64,
    pub networks: Vec<NetworkSource>,
    pub multiple_networks: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub schema_version: u32,
    pub buckets: BTreeMap<u64, MinuteBucket>,
    pub last_wall_ms: u64,
    pub clock_discontinuities: u64,
}
impl Default for Archive {
    fn default() -> Self {
        Self {
            schema_version: 1,
            buckets: BTreeMap::new(),
            last_wall_ms: 0,
            clock_discontinuities: 0,
        }
    }
}
#[derive(Default)]
pub struct TrafficTotals {
    pub received: u64,
    pub transmitted: u64,
    pub covered_ms: u64,
    pub networks: Vec<String>,
}
impl Archive {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.buckets.len() > MAX_BUCKETS {
            return Err("历史版本或数量不受支持".into());
        }
        for (&at, b) in &self.buckets {
            if at % MINUTE != 0
                || at > self.last_wall_ms
                || !b.cpu.valid()
                || !b.memory.valid()
                || b.network_ms > MINUTE
                || b.networks.len() > 4
                || b.networks
                    .iter()
                    .any(|n| n.id.len() > 512 || n.name.len() > 512)
            {
                return Err("历史数据校验失败".into());
            }
        }
        Ok(())
    }
    pub fn prune(&mut self, now: u64) {
        let cutoff = now.saturating_sub(DAY) / MINUTE * MINUTE;
        self.buckets.retain(|at, _| *at >= cutoff);
        while self.buckets.len() > MAX_BUCKETS {
            self.buckets.pop_first();
        }
    }
    pub fn accept(&mut self, input: ArchiveInput) {
        if input.wall_ms <= self.last_wall_ms {
            self.clock_discontinuities = self.clock_discontinuities.saturating_add(1);
            return;
        }
        let available = input.wall_ms - self.last_wall_ms;
        let duration = input.elapsed_ms.min(available);
        if duration > 0 && duration <= 15_000 {
            for (at, ms) in segments(input.wall_ms, duration) {
                let bucket = self.buckets.entry(at).or_default();
                if let Some(v) = input.cpu {
                    bucket.cpu.add(v, ms);
                }
                if let Some(v) = input.memory {
                    bucket.memory.add(v, ms);
                }
            }
        }
        if let (Some(delta), Some(source)) = (input.traffic, input.network) {
            let duration = delta.elapsed_ms.min(available);
            if duration > 0 && duration <= 15_000 {
                let parts = segments(input.wall_ms, duration);
                let (mut received, mut transmitted) = (0, 0);
                for (index, (at, ms)) in parts.iter().enumerate() {
                    let last = index + 1 == parts.len();
                    let read = if last {
                        delta.received - received
                    } else {
                        (delta.received as u128 * *ms as u128 / duration as u128) as u64
                    };
                    let write = if last {
                        delta.transmitted - transmitted
                    } else {
                        (delta.transmitted as u128 * *ms as u128 / duration as u128) as u64
                    };
                    received += read;
                    transmitted += write;
                    let bucket = self.buckets.entry(*at).or_default();
                    bucket.received = bucket.received.saturating_add(read);
                    bucket.transmitted = bucket.transmitted.saturating_add(write);
                    bucket.network_ms = (bucket.network_ms + ms).min(MINUTE);
                    if !bucket.networks.iter().any(|n| n.id == source.id) {
                        if bucket.networks.len() < 4 {
                            bucket.networks.push(NetworkSource {
                                id: source.id.chars().take(128).collect(),
                                name: source.name.chars().take(128).collect(),
                            });
                        }
                        bucket.multiple_networks = bucket.networks.len() > 1;
                    }
                }
            }
        }
        self.last_wall_ms = input.wall_ms;
        self.prune(input.wall_ms);
    }
    pub fn totals(&self, start: u64, end: u64) -> TrafficTotals {
        let mut total = TrafficTotals::default();
        // Callers request minute-aligned boundaries, including local midnight.
        for (_, b) in self.buckets.range(start..end) {
            total.received = total.received.saturating_add(b.received);
            total.transmitted = total.transmitted.saturating_add(b.transmitted);
            total.covered_ms += b.network_ms;
            for n in &b.networks {
                if total.networks.len() < 64 && !total.networks.contains(&n.name) {
                    total.networks.push(n.name.clone());
                }
            }
        }
        total
    }
}
fn segments(end: u64, duration: u64) -> Vec<(u64, u64)> {
    let mut start = end.saturating_sub(duration);
    let mut result = Vec::with_capacity(2);
    while start < end {
        let at = start / MINUTE * MINUTE;
        let through = end.min(at.saturating_add(MINUTE));
        result.push((at, through - start));
        start = through;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(wall: u64, duration: u64) -> ArchiveInput {
        ArchiveInput {
            wall_ms: wall,
            elapsed_ms: duration,
            cpu: Some(20.),
            memory: Some(40.),
            traffic: Some(TrafficDelta {
                received: 11,
                transmitted: 7,
                elapsed_ms: duration,
            }),
            network: Some(NetworkSource {
                id: "ethernet".into(),
                name: "以太网".into(),
            }),
        }
    }
    #[test]
    fn splits_midnight_without_losing_bytes_and_restores_partial_minute() {
        let mut a = Archive::default();
        a.accept(input(DAY + 500, 1000));
        assert_eq!(a.buckets.len(), 2);
        let yesterday = a.totals(DAY - MINUTE, DAY);
        let today = a.totals(DAY, DAY + MINUTE);
        assert_eq!(yesterday.received + today.received, 11);
        assert_eq!(today.received, 6);
        assert_eq!(today.transmitted, 4);
        assert_eq!(today.covered_ms, 500);
        assert_eq!(a.buckets[&DAY].cpu.average(), Some(20.));
        let mut restored: Archive =
            serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        let mut next = input(DAY + 1500, 1000);
        next.traffic = None;
        restored.accept(next);
        assert_eq!(restored.totals(DAY, DAY + MINUTE).received, 6);
        assert_eq!(restored.buckets[&DAY].cpu.covered_ms, 1500);
        restored.validate().unwrap();
    }
    #[test]
    fn gaps_clock_reversal_switches_and_retention() {
        let mut a = Archive::default();
        a.accept(input(MINUTE, 1000));
        let mut next = input(MINUTE + 1000, 1000);
        next.network.as_mut().unwrap().id = "wifi".into();
        a.accept(next);
        a.accept(input(MINUTE + 300_000, 300_000));
        assert_eq!(a.buckets.len(), 2);
        a.accept(input(1, 1000));
        assert_eq!(a.clock_discontinuities, 1);
        for i in 10..3000 {
            a.accept(input(i * MINUTE, 1000));
        }
        assert!(a.buckets.len() <= MAX_BUCKETS);
        assert!(*a.buckets.first_key_value().unwrap().0 >= a.last_wall_ms - DAY - MINUTE);
        a.validate().unwrap();
    }
}
