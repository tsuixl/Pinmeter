use crate::archive::{DAY, MAX_BUCKETS, MINUTE};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_APPS: usize = 128;
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
pub const UNKNOWN: &str = "unknown";
pub const OTHER: &str = "other";

/// Raw byte deltas from one accepted application-network window, never cumulative snapshots.
#[derive(Clone, Debug)]
pub struct AppWindow {
    pub generation: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub complete: bool,
    pub apps: BTreeMap<String, [u64; 2]>,
    pub unknown: [u64; 2],
    pub other: [u64; 2],
}
#[derive(Clone, Debug)]
pub struct AppHistoryInput {
    pub session: String,
    pub wall_ms: u64,
    pub window: AppWindow,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub path: String,
    pub name: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Minute {
    pub apps: BTreeMap<u16, [u64; 2]>,
    pub unknown: [u64; 2],
    pub other: [u64; 2],
    pub observed_ms: u64,
    pub covered_ms: u64,
    pub incomplete: bool,
    pub limited: bool,
    pub skipped: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppHistory {
    pub schema_version: u32,
    pub apps: BTreeMap<u16, Identity>,
    pub minutes: BTreeMap<u64, Minute>,
    pub last_wall_ms: u64,
    pub clock_discontinuities: u64,
    #[serde(skip)]
    last_delivery: Option<(String, u64, u64)>,
}
impl Default for AppHistory {
    fn default() -> Self {
        Self {
            schema_version: 1,
            apps: BTreeMap::new(),
            minutes: BTreeMap::new(),
            last_wall_ms: 0,
            clock_discontinuities: 0,
            last_delivery: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub path: String,
    pub bytes: [u128; 2],
}
#[derive(Default)]
pub struct Summary {
    pub rows: Vec<Row>,
    pub bytes: [u128; 2],
    pub covered_ms: u64,
    pub observed_ms: u64,
    pub incomplete: bool,
    pub limited: bool,
    pub skipped: u64,
}
pub struct Point {
    pub at_ms: u64,
    pub download: Option<f64>,
    pub upload: Option<f64>,
}
impl AppHistory {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.apps.len() > MAX_APPS
            || self.minutes.len() > MAX_BUCKETS
        {
            return Err("应用历史版本或数量无效".into());
        }
        if self.apps.iter().any(|(id, a)| {
            *id as usize >= MAX_APPS
                || a.path.is_empty()
                || a.path.len() > 4096
                || a.path.contains('\0')
                || a.name.len() > 512
        }) {
            return Err("应用历史身份无效".into());
        }
        let unique: BTreeSet<_> = self.apps.values().map(|a| &a.path).collect();
        if unique.len() != self.apps.len() {
            return Err("应用历史身份重复".into());
        }
        for (&at, m) in &self.minutes {
            if at % MINUTE != 0
                || at > self.last_wall_ms
                || m.observed_ms > MINUTE
                || m.covered_ms > m.observed_ms
                || m.apps.len() > MAX_APPS
                || m.apps.keys().any(|id| !self.apps.contains_key(id))
            {
                return Err("应用历史分钟数据无效".into());
            }
        }
        let used: BTreeSet<_> = self
            .minutes
            .values()
            .flat_map(|m| m.apps.keys().copied())
            .collect();
        if self.apps.keys().any(|id| !used.contains(id)) {
            return Err("应用历史存在未引用的身份".into());
        }
        Ok(())
    }
    pub fn prune(&mut self, now: u64) -> bool {
        let before = (self.minutes.len(), self.apps.len());
        let cutoff = now.saturating_sub(DAY) / MINUTE * MINUTE;
        self.minutes.retain(|at, _| *at >= cutoff);
        while self.minutes.len() > MAX_BUCKETS {
            self.minutes.pop_first();
        }
        if self.minutes.len() != before.0 {
            let used: BTreeSet<_> = self
                .minutes
                .values()
                .flat_map(|m| m.apps.keys().copied())
                .collect();
            self.apps.retain(|id, _| used.contains(id));
        }
        before != (self.minutes.len(), self.apps.len())
    }
    fn slot(&mut self, path: &str) -> Option<u16> {
        if path.is_empty() || path.len() > 4096 || path.contains('\0') {
            return None;
        }
        if let Some((&id, _)) = self.apps.iter().find(|(_, a)| a.path == path) {
            return Some(id);
        }
        let id = (0..MAX_APPS as u16).find(|id| !self.apps.contains_key(id))?;
        self.apps.insert(
            id,
            Identity {
                path: path.into(),
                name: path
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or(path)
                    .chars()
                    .take(128)
                    .collect(),
            },
        );
        Some(id)
    }
    pub fn accept(&mut self, input: AppHistoryInput) -> bool {
        let mut w = input.window;
        if self
            .last_delivery
            .as_ref()
            .is_some_and(|(session, generation, seq)| {
                *session == input.session
                    && (w.generation < *generation
                        || (w.generation == *generation && w.sequence <= *seq))
            })
        {
            return false;
        }
        if self
            .last_delivery
            .as_ref()
            .is_some_and(|(session, generation, sequence)| {
                *session == input.session
                    && *generation == w.generation
                    && w.sequence != sequence.saturating_add(1)
            })
        {
            w.complete = false;
        }
        self.last_delivery = Some((input.session, w.generation, w.sequence));
        if input.wall_ms <= self.last_wall_ms {
            self.clock_discontinuities = self.clock_discontinuities.saturating_add(1);
            return true;
        }
        self.prune(input.wall_ms);
        let duration = w.elapsed_ms.min(input.wall_ms - self.last_wall_ms);
        self.last_wall_ms = input.wall_ms;
        // Do not invent a date allocation for a suspended or unbounded collection window.
        if duration == 0 || w.elapsed_ms > 15_000 || w.apps.len() > MAX_APPS {
            let minute = self
                .minutes
                .entry(input.wall_ms / MINUTE * MINUTE)
                .or_default();
            minute.incomplete = true;
            minute.skipped = minute.skipped.saturating_add(1);
            return true;
        }
        let mut rows = BTreeMap::new();
        let mut other = w.other;
        let mut limited = other != [0, 0];
        for (path, bytes) in w.apps {
            if bytes == [0, 0] {
                continue;
            }
            if let Some(slot) = self.slot(&path) {
                rows.insert(slot, bytes);
            } else {
                add(&mut other, bytes);
                limited = true;
            }
        }
        let start = input.wall_ms.saturating_sub(duration);
        let parts = if start / MINUTE == (input.wall_ms - 1) / MINUTE {
            vec![(start / MINUTE * MINUTE, duration)]
        } else {
            let boundary = (start / MINUTE + 1) * MINUTE;
            vec![
                (start / MINUTE * MINUTE, boundary - start),
                (boundary, input.wall_ms - boundary),
            ]
        };
        for (index, (at, ms)) in parts.iter().enumerate() {
            let take = |bytes: [u64; 2]| split(bytes, parts[0].1, duration, index);
            let m = self.minutes.entry(*at).or_default();
            m.observed_ms = (m.observed_ms + ms).min(MINUTE);
            if w.complete {
                m.covered_ms = (m.covered_ms + ms).min(MINUTE);
            }
            m.incomplete |= !w.complete;
            m.limited |= limited;
            m.incomplete |= add(&mut m.unknown, take(w.unknown));
            m.incomplete |= add(&mut m.other, take(other));
            for (&id, &bytes) in &rows {
                m.incomplete |= add(m.apps.entry(id).or_default(), take(bytes));
            }
        }
        true
    }
    pub fn summarize(&self, start: u64, end: u64) -> Summary {
        let mut result = Summary::default();
        let mut apps = BTreeMap::<u16, [u128; 2]>::new();
        let mut unknown = [0u128; 2];
        let mut other = [0u128; 2];
        for (_, m) in self.minutes.range(start..end) {
            result.covered_ms += m.covered_ms;
            result.observed_ms += m.observed_ms;
            result.incomplete |= m.incomplete;
            result.limited |= m.limited;
            result.skipped = result.skipped.saturating_add(m.skipped);
            for i in 0..2 {
                unknown[i] += m.unknown[i] as u128;
                other[i] += m.other[i] as u128;
            }
            for (&id, bytes) in &m.apps {
                let total = apps.entry(id).or_default();
                for i in 0..2 {
                    total[i] += bytes[i] as u128;
                }
            }
        }
        for (slot, bytes) in apps {
            if let Some(app) = self.apps.get(&slot)
                && bytes != [0, 0]
            {
                result.rows.push(Row {
                    id: format!("app:{}", app.path),
                    name: app.name.clone(),
                    path: app.path.clone(),
                    bytes,
                });
            }
        }
        result.rows.sort_by(|a, b| {
            (b.bytes[0] + b.bytes[1])
                .cmp(&(a.bytes[0] + a.bytes[1]))
                .then_with(|| a.id.cmp(&b.id))
        });
        for (id, name, bytes) in [(OTHER, "其他已归属", other), (UNKNOWN, "未归属", unknown)]
        {
            if bytes != [0, 0] {
                result.rows.push(Row {
                    id: id.into(),
                    name: name.into(),
                    path: String::new(),
                    bytes,
                });
            }
        }
        for row in &result.rows {
            for i in 0..2 {
                result.bytes[i] += row.bytes[i];
            }
        }
        result
    }
    pub fn identity(&self, id: &str) -> Option<Identity> {
        match id {
            UNKNOWN => Some(Identity {
                path: String::new(),
                name: "未归属".into(),
            }),
            OTHER => Some(Identity {
                path: String::new(),
                name: "其他已归属".into(),
            }),
            _ => self
                .apps
                .values()
                .find(|a| Some(a.path.as_str()) == id.strip_prefix("app:"))
                .cloned(),
        }
    }
    pub fn series(&self, id: &str, start: u64, end: u64) -> Vec<Point> {
        let slot = self
            .apps
            .iter()
            .find(|(_, a)| Some(a.path.as_str()) == id.strip_prefix("app:"))
            .map(|(id, _)| *id);
        (start / MINUTE..=end / MINUTE)
            .take(MAX_BUCKETS)
            .map(|minute| {
                let at = minute * MINUTE;
                let m = self.minutes.get(&at);
                let values = m
                    .and_then(|m| {
                        if m.covered_ms == 0 || m.incomplete {
                            return None;
                        }
                        let bytes = match id {
                            UNKNOWN => m.unknown,
                            OTHER => m.other,
                            _ => *m.apps.get(&slot?).unwrap_or(&[0, 0]),
                        };
                        let mut rates = [None, None];
                        for i in 0..2 {
                            // An absent app is not a confirmed zero if this window has unattributed traffic.
                            if bytes[i] > 0
                                || id == UNKNOWN
                                || id == OTHER
                                || (m.unknown[i] == 0 && m.other[i] == 0)
                            {
                                rates[i] = Some(bytes[i] as f64 * 1000. / m.covered_ms as f64);
                            }
                        }
                        Some(rates)
                    })
                    .unwrap_or([None, None]);
                Point {
                    at_ms: (at + MINUTE).min(end),
                    download: values[0],
                    upload: values[1],
                }
            })
            .collect()
    }
}
fn add(target: &mut [u64; 2], bytes: [u64; 2]) -> bool {
    let mut overflow = false;
    for i in 0..2 {
        overflow |= target[i].checked_add(bytes[i]).is_none();
        target[i] = target[i].saturating_add(bytes[i]);
    }
    overflow
}
fn split(bytes: [u64; 2], first_ms: u64, total_ms: u64, index: usize) -> [u64; 2] {
    bytes.map(|n| {
        let first = (n as u128 * first_ms as u128 / total_ms as u128) as u64;
        if index == 0 { first } else { n - first }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(seq: u64, wall: u64, apps: &[(&str, u64, u64)]) -> AppHistoryInput {
        AppHistoryInput {
            session: "run1".into(),
            wall_ms: wall,
            window: AppWindow {
                generation: 1,
                sequence: seq,
                elapsed_ms: 1000,
                complete: true,
                apps: apps
                    .iter()
                    .map(|(p, d, u)| (p.to_string(), [*d, *u]))
                    .collect(),
                unknown: [0, 0],
                other: [0, 0],
            },
        }
    }
    #[test]
    fn deduplicates_windows_and_keeps_cross_midnight_totals_after_restart() {
        let mut h = AppHistory::default();
        let window = input(1, DAY + 500, &[("c:\\a.exe", 11, 7), ("d:\\a.exe", 9, 3)]);
        assert!(h.accept(window.clone()));
        assert!(!h.accept(window));
        assert_eq!(h.summarize(DAY - MINUTE, DAY + MINUTE).bytes, [20, 10]);
        assert_eq!(h.summarize(DAY, DAY + MINUTE).bytes, [11, 6]);
        assert_eq!(h.summarize(DAY, DAY + MINUTE).rows.len(), 2);
        let mut restored: AppHistory =
            serde_json::from_slice(&serde_json::to_vec(&h).unwrap()).unwrap();
        restored.validate().unwrap();
        let mut next = input(1, DAY + 3000, &[("c:\\a.exe", 5, 1)]);
        next.session = "run2".into();
        restored.accept(next);
        assert_eq!(restored.summarize(DAY, DAY + MINUTE).bytes, [16, 7]);
    }
    #[test]
    fn missing_ownership_loss_and_suspension_never_forge_zeroes() {
        let mut h = AppHistory::default();
        h.accept(input(1, MINUTE + 1000, &[("c:\\a.exe", 10, 0)]));
        let mut other = input(2, MINUTE * 2 + 1000, &[]);
        other.window.unknown = [42, 0];
        h.accept(other);
        let point = h.series("app:c:\\a.exe", MINUTE * 2, MINUTE * 3).remove(0);
        assert_eq!(point.download, None);
        assert_eq!(point.upload, Some(0.));
        let mut lost = input(3, MINUTE * 3 + 1000, &[("c:\\a.exe", 20, 0)]);
        lost.window.complete = false;
        h.accept(lost);
        assert!(h.summarize(MINUTE * 3, MINUTE * 4).incomplete);
        assert!(
            h.series("app:c:\\a.exe", MINUTE * 3, MINUTE * 4)
                .iter()
                .all(|p| p.download.is_none())
        );
        let mut suspended = input(4, MINUTE * 10, &[("c:\\a.exe", 9999, 0)]);
        suspended.window.elapsed_ms = MINUTE * 6;
        h.accept(suspended);
        assert_eq!(h.summarize(0, MINUTE * 11).bytes, [72, 0]);
        assert_eq!(h.summarize(0, MINUTE * 11).skipped, 1);
    }
    #[test]
    fn capacity_overflow_preserves_other_bytes_and_expires_identities() {
        let mut h = AppHistory::default();
        for n in 0..=MAX_APPS {
            h.accept(input(
                n as u64 + 1,
                1000 + n as u64 * 1000,
                &[(&format!("c:\\{n}.exe"), 10, 1)],
            ));
        }
        let total = h.summarize(0, MINUTE * 10);
        assert_eq!(h.apps.len(), MAX_APPS);
        assert!(total.limited);
        assert_eq!(
            total.bytes,
            [(MAX_APPS as u128 + 1) * 10, MAX_APPS as u128 + 1]
        );
        assert!(total.rows.iter().any(|r| r.id == OTHER));
        h.accept(input(1000, DAY + MINUTE * 10, &[("d:\\new.exe", 1, 0)]));
        assert_eq!(h.apps.len(), 1);
        assert!(h.identity("app:c:\\0.exe").is_none());
        h.validate().unwrap();
    }
    #[test]
    fn maximum_valid_file_stays_inside_budget() {
        let mut h = AppHistory {
            last_wall_ms: DAY,
            ..Default::default()
        };
        for id in 0..MAX_APPS as u16 {
            h.apps.insert(
                id,
                Identity {
                    path: format!("c:\\{}-{id}.exe", "a".repeat(3900)),
                    name: "a".repeat(128),
                },
            );
        }
        for m in 0..MAX_BUCKETS as u64 {
            h.minutes.insert(
                m * MINUTE,
                Minute {
                    apps: (0..MAX_APPS as u16).map(|id| (id, [u64::MAX; 2])).collect(),
                    unknown: [u64::MAX; 2],
                    other: [u64::MAX; 2],
                    observed_ms: MINUTE,
                    covered_ms: MINUTE,
                    ..Default::default()
                },
            );
        }
        h.validate().unwrap();
        assert!(serde_json::to_vec(&h).unwrap().len() < MAX_FILE_BYTES);
    }

    #[test]
    fn a_dropped_archive_delivery_marks_the_minute_incomplete() {
        let mut h = AppHistory::default();
        h.accept(input(1, 1000, &[("c:\\a.exe", 5, 0)]));
        h.accept(input(3, 3000, &[("c:\\a.exe", 7, 0)]));
        let summary = h.summarize(0, MINUTE);
        assert!(summary.incomplete);
        assert_eq!(summary.bytes, [12, 0]);
        assert_eq!(summary.covered_ms, 1000);
        assert!(
            h.series("app:c:\\a.exe", 0, MINUTE)
                .iter()
                .all(|p| p.download.is_none())
        );
    }
}
