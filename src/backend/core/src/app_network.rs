use crate::domain::{Failure, Status};
use std::collections::BTreeMap;

pub const MAX_APPS: usize = 128;
pub const MAX_PROCESSES: usize = 512;

#[derive(Clone, Debug)]
pub struct ProcessBytes {
    pub icon: Option<String>,
    pub pid: u32,
    pub started: String,
    pub path: String,
    pub download: u64,
    pub upload: u64,
}

#[derive(Clone, Debug)]
pub struct NetworkWindow {
    pub other_download: u64,
    pub other_upload: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub lost: u64,
    pub rows: Vec<ProcessBytes>,
    pub unknown_download: u64,
    pub unknown_upload: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Traffic {
    pub download: f64,
    pub upload: f64,
    pub received: u64,
    pub sent: u64,
    pub download_share: Option<f64>,
    pub upload_share: Option<f64>,
}
impl Traffic {
    fn clear_rates(&mut self) {
        self.download = 0.0;
        self.upload = 0.0;
        self.download_share = None;
        self.upload_share = None;
    }
    fn add(&mut self, download: u64, upload: u64, seconds: f64) {
        self.received = self.received.saturating_add(download);
        self.sent = self.sent.saturating_add(upload);
        self.download += download as f64 / seconds;
        self.upload += upload as f64 / seconds;
    }
}

#[derive(Clone, Debug)]
pub struct ProcessTraffic {
    pub id: String,
    pub pid: u32,
    /// Traffic was observed in this window, not a claim that the process is alive.
    pub observed: bool,
    pub traffic: Traffic,
}
#[derive(Clone, Debug)]
pub struct AppTraffic {
    pub icon: Option<String>,
    pub id: String,
    pub name: String,
    pub path: String,
    pub traffic: Traffic,
    pub processes: BTreeMap<String, ProcessTraffic>,
}

pub struct AppNetwork {
    pub generation: u64,
    pub revision: u64,
    pub requested: bool,
    pub status: &'static str,
    pub detail: String,
    pub incomplete: bool,
    pub limited: bool,
    pub apps: BTreeMap<String, AppTraffic>,
    pub unknown: Traffic,
    pub other: Traffic,
    sampled_at: u64,
    sequence: u64,
    lost: u64,
    process_count: usize,
    supported: bool,
}
impl Default for AppNetwork {
    fn default() -> Self {
        Self::new(false)
    }
}
impl AppNetwork {
    pub fn new(supported: bool) -> Self {
        Self {
            generation: 0,
            revision: 0,
            requested: false,
            status: if supported { "disabled" } else { "unsupported" },
            detail: if supported {
                "开始后持续统计应用流量；切页和最小化到托盘继续监控"
            } else {
                "此平台尚未接入应用网络采集"
            }
            .into(),
            incomplete: false,
            limited: false,
            apps: BTreeMap::new(),
            unknown: Traffic::default(),
            other: Traffic::default(),
            sampled_at: 0,
            sequence: 0,
            lost: 0,
            process_count: 0,
            supported,
        }
    }
    pub fn begin(&mut self, now: u64) -> Result<u64, String> {
        if !self.supported {
            return Err("此平台尚未接入应用网络采集".into());
        }
        if self.requested {
            return Ok(self.generation);
        }
        let generation = self.generation.wrapping_add(1);
        let revision = self.revision.wrapping_add(1);
        *self = Self::new(true);
        self.generation = generation;
        self.revision = revision;
        self.requested = true;
        self.sampled_at = now;
        self.status = "authorizing";
        self.detail = "正在启动应用网络采集".into();
        Ok(generation)
    }
    pub fn stop(&mut self) {
        if !self.requested {
            return;
        }
        self.requested = false;
        self.generation = self.generation.wrapping_add(1);
        self.revision = self.revision.wrapping_add(1);
        self.status = "disabled";
        self.detail = "本次监控已停止；再次开始将重新累计".into();
        self.clear_rates();
    }
    fn clear_rates(&mut self) {
        for app in self.apps.values_mut() {
            app.traffic.clear_rates();
            for process in app.processes.values_mut() {
                process.traffic.clear_rates();
                process.observed = false;
            }
        }
        self.unknown.clear_rates();
        self.other.clear_rates();
    }
    pub fn status_at(&self, now: u64, interval: u64) -> (&str, &str) {
        let timeout = if self.status == "authorizing" {
            125_000
        } else {
            interval * 3 + 1000
        };
        if self.requested && now.saturating_sub(self.sampled_at) > timeout {
            ("stale", "未在预期时间内获得有效应用流量")
        } else {
            (self.status, &self.detail)
        }
    }
    pub fn fail(&mut self, generation: u64, failure: &Failure) {
        if generation != self.generation || !self.requested {
            return;
        }
        self.requested = false;
        self.status = match failure.status {
            Status::PermissionDenied => "permission_denied",
            Status::Unsupported => "unsupported",
            _ => "failed",
        };
        self.detail = failure.reason.clone();
        self.incomplete = self.sequence > 0;
        self.revision = self.revision.wrapping_add(1);
        self.clear_rates();
    }
    pub fn accept(&mut self, generation: u64, window: &NetworkWindow, now: u64, interval: u64) {
        if generation != self.generation || !self.requested || window.sequence <= self.sequence {
            return;
        }
        let first = self.sequence == 0;
        let gap = !first
            && (window.sequence != self.sequence + 1
                || window.elapsed_ms > interval * 3 + 1000
                || now.saturating_sub(self.sampled_at) > interval * 3 + 1000);
        let lost = window.lost != self.lost;
        self.incomplete |= lost || gap;
        self.sequence = window.sequence;
        self.lost = window.lost;
        self.sampled_at = now;
        self.revision = self.revision.wrapping_add(1);
        self.clear_rates();
        let valid = !first && !gap && !lost && window.elapsed_ms >= 100;
        self.status = if lost || gap {
            "incomplete"
        } else if !valid {
            "warming"
        } else {
            "normal"
        };
        self.detail = match self.status {
            "incomplete" => "采集存在缺失，本窗口不计算精确占比",
            "warming" => "正在建立应用速率窗口",
            _ => "",
        }
        .into();
        let seconds = window.elapsed_ms.max(1) as f64 / 1000.0;
        self.unknown
            .add(window.unknown_download, window.unknown_upload, seconds);
        self.other
            .add(window.other_download, window.other_upload, seconds);
        self.limited |= window.other_download > 0 || window.other_upload > 0;
        for row in &window.rows {
            let key = row.path.replace('/', "\\").to_lowercase();
            if !self.apps.contains_key(&key) && self.apps.len() >= MAX_APPS {
                self.other.add(row.download, row.upload, seconds);
                self.limited = true;
                continue;
            }
            let app = self.apps.entry(key.clone()).or_insert_with(|| AppTraffic {
                icon: None,
                id: key,
                name: row
                    .path
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or(&row.path)
                    .to_string(),
                path: row.path.clone(),
                traffic: Traffic::default(),
                processes: BTreeMap::new(),
            });
            if row.icon.is_some() {
                app.icon.clone_from(&row.icon);
            }
            app.traffic.add(row.download, row.upload, seconds);
            let id = format!("{}:{}", row.pid, row.started);
            if !app.processes.contains_key(&id) && self.process_count >= MAX_PROCESSES {
                self.limited = true;
                continue;
            }
            let process = app.processes.entry(id.clone()).or_insert_with(|| {
                self.process_count += 1;
                ProcessTraffic {
                    id,
                    pid: row.pid,
                    observed: false,
                    traffic: Traffic::default(),
                }
            });
            process.observed = row.download > 0 || row.upload > 0;
            process.traffic.add(row.download, row.upload, seconds);
        }
        if !valid {
            self.clear_rates();
            return;
        }
        let download =
            self.apps.values().map(|a| a.traffic.download).sum::<f64>() + self.other.download;
        let upload = self.apps.values().map(|a| a.traffic.upload).sum::<f64>() + self.other.upload;
        for traffic in self
            .apps
            .values_mut()
            .map(|a| &mut a.traffic)
            .chain(std::iter::once(&mut self.other))
        {
            traffic.download_share = (download > 0.0).then(|| traffic.download / download * 100.0);
            traffic.upload_share = (upload > 0.0).then(|| traffic.upload / upload * 100.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(pid: u32, started: &str, path: &str, download: u64) -> ProcessBytes {
        ProcessBytes {
            icon: None,
            pid,
            started: started.into(),
            path: path.into(),
            download,
            upload: 0,
        }
    }
    fn sample(sequence: u64, rows: Vec<ProcessBytes>) -> NetworkWindow {
        NetworkWindow {
            other_download: 0,
            other_upload: 0,
            sequence,
            elapsed_ms: 1000,
            lost: 0,
            rows,
            unknown_download: 100,
            unknown_upload: 0,
        }
    }
    #[test]
    fn groups_paths_and_preserves_process_identity_and_denominator() {
        let mut state = AppNetwork::new(true);
        let generation = state.begin(0).unwrap();
        state.accept(generation, &sample(1, vec![]), 0, 1000);
        state.accept(
            generation,
            &sample(
                2,
                vec![
                    row(1, "1", "C:\\a.exe", 100),
                    row(2, "1", "c:\\A.exe", 200),
                    row(3, "1", "C:\\b.exe", 300),
                ],
            ),
            1000,
            1000,
        );
        let app = &state.apps["c:\\a.exe"];
        assert_eq!(app.traffic.download, 300.0);
        assert_eq!(app.traffic.download_share, Some(50.0));
        assert_eq!(app.processes.len(), 2);
        state.accept(
            generation,
            &sample(3, vec![row(1, "2", "C:\\a.exe", 20)]),
            2000,
            1000,
        );
        let app = &state.apps["c:\\a.exe"];
        assert_eq!(app.processes.len(), 3);
        assert!(!app.processes["1:1"].observed);
        assert_eq!(app.traffic.received, 320);
        assert_eq!(state.status_at(7000, 1000).0, "stale");
    }
    #[test]
    fn lost_windows_and_old_sessions_do_not_forge_complete_data() {
        let mut state = AppNetwork::new(true);
        let generation = state.begin(0).unwrap();
        state.accept(generation, &sample(1, vec![]), 0, 1000);
        let mut window = sample(2, vec![row(1, "1", "a", 100)]);
        window.lost = 2;
        state.accept(generation, &window, 1000, 1000);
        assert!(state.incomplete);
        assert_eq!(state.status, "incomplete");
        assert_eq!(state.apps["a"].traffic.download_share, None);
        state.accept(generation, &window, 2000, 1000);
        assert_eq!(state.apps["a"].traffic.received, 100);
        state.stop();
        let next = state.begin(2000).unwrap();
        state.accept(
            generation,
            &sample(4, vec![row(1, "1", "a", 999)]),
            3000,
            1000,
        );
        assert!(state.apps.is_empty());
        assert_ne!(next, generation);
    }
    #[test]
    fn bounds_keep_overflow_in_denominator() {
        let mut state = AppNetwork::new(true);
        let generation = state.begin(0).unwrap();
        state.accept(generation, &sample(1, vec![]), 0, 1000);
        state.accept(
            generation,
            &sample(
                2,
                (0..130).map(|i| row(i, "1", &i.to_string(), 100)).collect(),
            ),
            1000,
            1000,
        );
        assert_eq!(state.apps.len(), MAX_APPS);
        assert_eq!(state.other.download, 200.0);
        assert!((state.apps["0"].traffic.download_share.unwrap() - 100.0 / 130.0).abs() < 1e-10);
    }
}
