use crate::domain::TrafficDelta;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MINUTE: u64 = 60_000;
pub const DAY: u64 = 24 * 60 * MINUTE;
pub const MAX_BUCKETS: usize = 1441;
pub const QUARTER_HOUR: u64 = 15 * MINUTE;
pub const HOUR: u64 = 60 * MINUTE;
pub const WEEK: u64 = 7 * DAY;
pub const MONTH: u64 = 30 * DAY;
pub const MAX_QUARTERS: usize = 673;
pub const MAX_HOURS: usize = 721;
pub const MAX_NETWORKS: usize = 4;
pub const MAX_NETWORK_TEXT_BYTES: usize = 512;
// Every byte in a legal source string may require a six-byte JSON escape.
// 2835 buckets * (4 sources * 2 fields * 512 bytes * 6 + 1024 bytes for
// numeric fields, field names and map keys) + 1024 bytes of archive metadata
// bounds the current schema at 72,577,024 bytes. Keep the 80 MiB hard limit in
// one place for readers, writers and UI; the complete worst-case file is tested.
pub const MAX_SERIALIZED_BYTES_BOUND: usize = (MAX_BUCKETS + MAX_QUARTERS + MAX_HOURS)
    * (MAX_NETWORKS * 2 * MAX_NETWORK_TEXT_BYTES * 6 + 1024)
    + 1024;
pub const MAX_FILE_BYTES: usize = 80 * 1024 * 1024;
pub fn resolution(range_ms: u64) -> u64 {
    if range_ms <= DAY {
        MINUTE
    } else if range_ms <= WEEK {
        QUARTER_HOUR
    } else {
        HOUR
    }
}

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
    #[serde(default)]
    pub max_at_ms: Option<u64>,
}
impl Aggregate {
    pub fn average(&self) -> Option<f64> {
        (self.covered_ms > 0).then(|| self.weighted_sum / self.covered_ms as f64)
    }
    fn add(&mut self, value: f64, ms: u64, at_ms: u64) {
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
        if self.max.is_none_or(|old| value > old) {
            self.max = Some(value);
            self.max_at_ms = Some(at_ms);
        }
    }
    fn valid(&self, resolution_ms: u64) -> bool {
        self.covered_ms <= resolution_ms
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peak {
    pub value: f64,
    pub at_ms: u64,
}
fn keep_peak(current: &mut Option<Peak>, value: f64, at_ms: u64) {
    if value.is_finite() && value >= 0. && current.as_ref().is_none_or(|old| value > old.value) {
        *current = Some(Peak { value, at_ms });
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
    #[serde(default)]
    pub download_peak: Option<Peak>,
    #[serde(default)]
    pub upload_peak: Option<Peak>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub schema_version: u32,
    pub buckets: BTreeMap<u64, MinuteBucket>,
    #[serde(default)]
    pub quarters: BTreeMap<u64, MinuteBucket>,
    #[serde(default)]
    pub hours: BTreeMap<u64, MinuteBucket>,
    pub last_wall_ms: u64,
    pub clock_discontinuities: u64,
    #[serde(default)]
    pub cleared_before_ms: u64,
}
impl Default for Archive {
    fn default() -> Self {
        Self {
            schema_version: 2,
            buckets: BTreeMap::new(),
            quarters: BTreeMap::new(),
            hours: BTreeMap::new(),
            last_wall_ms: 0,
            clock_discontinuities: 0,
            cleared_before_ms: 0,
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
    pub fn cleared(at_ms: u64) -> Self {
        Self {
            last_wall_ms: at_ms,
            cleared_before_ms: at_ms,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.schema_version, 1 | 2)
            || self.buckets.len() > MAX_BUCKETS
            || self.quarters.len() > MAX_QUARTERS
            || self.hours.len() > MAX_HOURS
        {
            return Err("历史版本或数量不受支持".into());
        }
        for (buckets, resolution_ms) in [
            (&self.buckets, MINUTE),
            (&self.quarters, QUARTER_HOUR),
            (&self.hours, HOUR),
        ] {
            for (&at, b) in buckets {
                if at % resolution_ms != 0
                    || at > self.last_wall_ms
                    || !b.cpu.valid(resolution_ms)
                    || !b.memory.valid(resolution_ms)
                    || b.network_ms > resolution_ms
                    || b.networks.len() > MAX_NETWORKS
                    || b.networks.iter().any(|n| {
                        n.id.len() > MAX_NETWORK_TEXT_BYTES || n.name.len() > MAX_NETWORK_TEXT_BYTES
                    })
                    || b.cpu
                        .max_at_ms
                        .into_iter()
                        .chain(b.memory.max_at_ms)
                        .any(|time| time > self.last_wall_ms)
                    || b.download_peak.iter().chain(&b.upload_peak).any(|peak| {
                        !peak.value.is_finite() || peak.value < 0. || peak.at_ms > self.last_wall_ms
                    })
                {
                    return Err("历史数据校验失败".into());
                }
            }
        }
        Ok(())
    }
    pub fn migrate(&mut self) {
        if self.schema_version == 1 {
            self.quarters.clear();
            self.hours.clear();
            for (&at, bucket) in &self.buckets {
                merge_bucket(
                    self.quarters
                        .entry(at / QUARTER_HOUR * QUARTER_HOUR)
                        .or_default(),
                    bucket,
                );
                merge_bucket(self.hours.entry(at / HOUR * HOUR).or_default(), bucket);
            }
            self.schema_version = 2;
        }
    }
    pub fn layer(&self, resolution_ms: u64) -> &BTreeMap<u64, MinuteBucket> {
        match resolution_ms {
            QUARTER_HOUR => &self.quarters,
            HOUR => &self.hours,
            _ => &self.buckets,
        }
    }
    pub fn prune(&mut self, now: u64) {
        let cutoff = now.saturating_sub(DAY) / MINUTE * MINUTE;
        self.buckets.retain(|at, _| *at >= cutoff);
        while self.buckets.len() > MAX_BUCKETS {
            self.buckets.pop_first();
        }
        prune_layer(&mut self.quarters, now, WEEK, QUARTER_HOUR, MAX_QUARTERS);
        prune_layer(&mut self.hours, now, MONTH, HOUR, MAX_HOURS);
    }
    pub fn accept(&mut self, input: ArchiveInput) {
        if input.wall_ms <= self.cleared_before_ms
            || input.wall_ms.saturating_sub(input.elapsed_ms) < self.cleared_before_ms
            || input.traffic.as_ref().is_some_and(|delta| {
                input.wall_ms.saturating_sub(delta.elapsed_ms) < self.cleared_before_ms
            })
        {
            return;
        }
        if input.wall_ms <= self.last_wall_ms {
            self.clock_discontinuities = self.clock_discontinuities.saturating_add(1);
            return;
        }
        let available = input.wall_ms - self.last_wall_ms;
        let mut incoming = BTreeMap::<u64, MinuteBucket>::new();
        let duration = input.elapsed_ms.min(available);
        if duration > 0 && duration <= 15_000 {
            for (at, ms) in segments(input.wall_ms, duration) {
                let bucket = incoming.entry(at).or_default();
                if let Some(v) = input.cpu {
                    bucket.cpu.add(v, ms, input.wall_ms);
                }
                if let Some(v) = input.memory {
                    bucket.memory.add(v, ms, input.wall_ms);
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
                    let bucket = incoming.entry(*at).or_default();
                    bucket.received = bucket.received.saturating_add(read);
                    bucket.transmitted = bucket.transmitted.saturating_add(write);
                    bucket.network_ms = (bucket.network_ms + ms).min(MINUTE);
                    keep_peak(
                        &mut bucket.download_peak,
                        delta.received as f64 * 1000. / delta.elapsed_ms as f64,
                        input.wall_ms,
                    );
                    keep_peak(
                        &mut bucket.upload_peak,
                        delta.transmitted as f64 * 1000. / delta.elapsed_ms as f64,
                        input.wall_ms,
                    );
                    if !bucket.networks.iter().any(|n| n.id == source.id) {
                        if bucket.networks.len() < MAX_NETWORKS {
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
        for (at, bucket) in incoming {
            merge_bucket(self.buckets.entry(at).or_default(), &bucket);
            merge_bucket(
                self.quarters
                    .entry(at / QUARTER_HOUR * QUARTER_HOUR)
                    .or_default(),
                &bucket,
            );
            merge_bucket(self.hours.entry(at / HOUR * HOUR).or_default(), &bucket);
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
pub fn prune_layer<T>(
    layer: &mut BTreeMap<u64, T>,
    now: u64,
    retention: u64,
    resolution_ms: u64,
    limit: usize,
) {
    let cutoff = now.saturating_sub(retention) / resolution_ms * resolution_ms;
    layer.retain(|at, _| *at >= cutoff);
    while layer.len() > limit {
        layer.pop_first();
    }
}
fn merge_aggregate(target: &mut Aggregate, source: &Aggregate) {
    target.weighted_sum += source.weighted_sum;
    target.covered_ms += source.covered_ms;
    if let Some(min) = source.min {
        target.min = Some(target.min.map_or(min, |old| old.min(min)));
    }
    if let Some(max) = source.max
        && target.max.is_none_or(|old| max > old)
    {
        target.max = Some(max);
        target.max_at_ms = source.max_at_ms;
    }
}
fn merge_bucket(target: &mut MinuteBucket, source: &MinuteBucket) {
    merge_aggregate(&mut target.cpu, &source.cpu);
    merge_aggregate(&mut target.memory, &source.memory);
    target.received = target.received.saturating_add(source.received);
    target.transmitted = target.transmitted.saturating_add(source.transmitted);
    target.network_ms += source.network_ms;
    for network in &source.networks {
        if !target.networks.iter().any(|old| old.id == network.id) {
            target.multiple_networks |= !target.networks.is_empty();
            if target.networks.len() < MAX_NETWORKS {
                target.networks.push(network.clone());
            }
        }
    }
    target.multiple_networks |= source.multiple_networks;
    for (to, from) in [
        (&mut target.download_peak, &source.download_peak),
        (&mut target.upload_peak, &source.upload_peak),
    ] {
        if let Some(peak) = from {
            keep_peak(to, peak.value, peak.at_ms);
        }
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
    #[test]
    fn archive_file_budget_covers_the_valid_network_string_bounds() {
        const { assert!(MAX_FILE_BYTES >= MAX_SERIALIZED_BYTES_BOUND) };
        let mut archive = Archive {
            last_wall_ms: MINUTE,
            ..Default::default()
        };
        let bucket = MinuteBucket {
            networks: (0..MAX_NETWORKS)
                .map(|index| NetworkSource {
                    id: char::from_u32(index as u32)
                        .unwrap()
                        .to_string()
                        .repeat(MAX_NETWORK_TEXT_BYTES),
                    name: "\u{1}".repeat(MAX_NETWORK_TEXT_BYTES),
                })
                .collect(),
            ..Default::default()
        };
        archive.buckets.insert(0, bucket);
        archive.validate().unwrap();
        archive.buckets.get_mut(&0).unwrap().networks[0]
            .name
            .push('a');
        assert!(
            archive.validate().is_err(),
            "storage bounds must follow validated byte limits"
        );
        archive.buckets.get_mut(&0).unwrap().networks[0].name.pop();
        archive
            .buckets
            .get_mut(&0)
            .unwrap()
            .networks
            .push(NetworkSource {
                id: "extra".into(),
                name: "extra".into(),
            });
        assert!(archive.validate().is_err());
    }
    #[test]
    fn coarse_layers_keep_weighted_samples_peaks_and_exact_bytes() {
        let mut archive = Archive::default();
        let mut high = input(HOUR + 1000, 1000);
        high.cpu = Some(100.);
        high.traffic.as_mut().unwrap().received = 100;
        archive.accept(high);
        let mut low = input(HOUR + 10000, 9000);
        low.cpu = Some(0.);
        low.traffic.as_mut().unwrap().received = 900;
        archive.accept(low);
        for resolution_ms in [MINUTE, QUARTER_HOUR, HOUR] {
            let bucket = &archive.layer(resolution_ms)[&HOUR];
            assert_eq!(bucket.cpu.average(), Some(10.));
            assert_eq!(bucket.cpu.max, Some(100.));
            assert_eq!(bucket.cpu.max_at_ms, Some(HOUR + 1000));
            assert_eq!(bucket.received, 1000);
            assert_eq!(bucket.network_ms, 10000);
            assert_eq!(bucket.download_peak.as_ref().unwrap().value, 100.);
        }
        archive.validate().unwrap();
        archive.prune(2 * DAY);
        assert!(archive.buckets.is_empty());
        assert!(!archive.hours.is_empty());
        archive.prune(8 * DAY);
        assert!(archive.quarters.is_empty());
        assert!(!archive.hours.is_empty());
        archive.prune(31 * DAY);
        assert!(archive.hours.is_empty());
    }
    #[test]
    fn legacy_peaks_do_not_invent_times_or_network_peaks() {
        let mut original = Archive::default();
        original.accept(input(HOUR + 1000, 1000));
        let mut json = serde_json::to_value(original).unwrap();
        json["schema_version"] = 1.into();
        json.as_object_mut().unwrap().remove("quarters");
        json.as_object_mut().unwrap().remove("hours");
        for bucket in json["buckets"].as_object_mut().unwrap().values_mut() {
            bucket["cpu"].as_object_mut().unwrap().remove("max_at_ms");
            bucket["memory"]
                .as_object_mut()
                .unwrap()
                .remove("max_at_ms");
            bucket.as_object_mut().unwrap().remove("download_peak");
            bucket.as_object_mut().unwrap().remove("upload_peak");
        }
        let mut archive: Archive = serde_json::from_value(json).unwrap();
        archive.validate().unwrap();
        archive.migrate();
        assert_eq!(archive.hours[&HOUR].cpu.max, Some(20.));
        assert_eq!(archive.hours[&HOUR].cpu.max_at_ms, None);
        assert!(archive.hours[&HOUR].download_peak.is_none());
        archive.validate().unwrap();
    }
    #[test]
    fn cleared_watermark_rejects_late_and_crossing_windows() {
        let mut archive = Archive::cleared(DAY + 1000);
        archive.accept(input(DAY + 500, 1000));
        archive.accept(input(DAY + 1500, 1000));
        assert!(archive.buckets.is_empty());
        archive.accept(input(DAY + 2500, 1000));
        assert_eq!(archive.totals(DAY, DAY + MINUTE).received, 11);
    }
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
