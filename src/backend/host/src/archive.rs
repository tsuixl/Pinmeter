use pinmeter_core::{
    archive::{Archive, ArchiveInput, DAY, MINUTE},
    ports::Clock,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use ts_rs::TS;
#[derive(Clone, Serialize, TS)]
pub struct MinuteBucketDto {
    pub at_ms: f64,
    pub cpu: Option<f64>,
    pub cpu_min: Option<f64>,
    pub cpu_max: Option<f64>,
    pub memory: Option<f64>,
    pub cpu_coverage_ms: f64,
    pub memory_coverage_ms: f64,
    pub download: Option<f64>,
    pub upload: Option<f64>,
    pub network_coverage_ms: f64,
    pub network_key: String,
}
#[derive(Clone, Serialize, TS)]
pub struct ArchiveSnapshotDto {
    pub loading: bool,
    pub notice: String,
    pub persistence_error: String,
    pub saved_at_ms: Option<f64>,
    pub lost_samples: String,
    pub clock_discontinuities: String,
    pub now_ms: f64,
    pub day_start_ms: f64,
    pub received: String,
    pub transmitted: String,
    pub network_coverage_ms: f64,
    pub networks: Vec<String>,
    pub buckets: Vec<MinuteBucketDto>,
}
struct Data {
    archive: Archive,
    loading: bool,
    notice: String,
    error: String,
    saved_at: Option<u64>,
}
pub struct Archives {
    tx: SyncSender<ArchiveInput>,
    data: Mutex<Data>,
    stop: AtomicBool,
    lost: AtomicU64,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Archives {
    pub fn start(path: PathBuf) -> Arc<Self> {
        let (tx, rx) = mpsc::sync_channel(64);
        let service = Arc::new(Self {
            tx,
            data: Mutex::new(Data {
                archive: Archive::default(),
                loading: true,
                notice: String::new(),
                error: String::new(),
                saved_at: None,
            }),
            stop: AtomicBool::new(false),
            lost: AtomicU64::new(0),
            worker: Mutex::new(None),
        });
        let s = service.clone();
        *service.worker.lock().unwrap() = Some(thread::spawn(move || {
            let store = pinmeter_platform::archive::ArchiveFile::new(path);
            let mut writable = true;
            {
                let mut data = s.data.lock().unwrap();
                match store.load() {
                    Ok(archive) => {
                        data.saved_at = (archive.last_wall_ms > 0).then_some(archive.last_wall_ms);
                        data.archive = archive;
                    }
                    Err(error) => match store.preserve_invalid() {
                        Ok(()) => {
                            data.notice = format!("{error}；原件已保留，从本次运行重新记录。")
                        }
                        Err(reason) => {
                            writable = false;
                            data.error = format!("{error}；{reason}");
                        }
                    },
                }
                data.loading = false;
            }
            let mut next = Instant::now() + Duration::from_secs(60);
            let mut dirty = false;
            loop {
                if let Ok(input) = rx.recv_timeout(Duration::from_millis(250)) {
                    s.data.lock().unwrap().archive.accept(input);
                    dirty = true;
                }
                for input in rx.try_iter() {
                    s.data.lock().unwrap().archive.accept(input);
                    dirty = true;
                }
                let stopping = s.stop.load(Ordering::Acquire);
                if dirty && writable && (stopping || Instant::now() >= next) {
                    let copy = s.data.lock().unwrap().archive.clone();
                    let result = store.save(&copy);
                    let mut data = s.data.lock().unwrap();
                    match result {
                        Ok(()) => {
                            data.error.clear();
                            data.saved_at = Some(copy.last_wall_ms);
                            dirty = false;
                        }
                        Err(e) => data.error = e,
                    }
                    next = Instant::now() + Duration::from_secs(60);
                }
                if stopping {
                    break;
                }
            }
        }));
        service
    }
    pub fn offer(&self, input: ArchiveInput) {
        if self.tx.try_send(input).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
    fn snapshot(&self, day_start: u64, now: u64) -> ArchiveSnapshotDto {
        let data = self.data.lock().unwrap();
        let a = &data.archive;
        let total = a.totals(day_start, now.max(day_start));
        ArchiveSnapshotDto {
            loading: data.loading,
            notice: data.notice.clone(),
            persistence_error: data.error.clone(),
            saved_at_ms: data.saved_at.map(|v| v as f64),
            lost_samples: self.lost.load(Ordering::Relaxed).to_string(),
            clock_discontinuities: a.clock_discontinuities.to_string(),
            now_ms: now as f64,
            day_start_ms: day_start as f64,
            received: total.received.to_string(),
            transmitted: total.transmitted.to_string(),
            network_coverage_ms: total.covered_ms as f64,
            networks: total.networks,
            buckets: a
                .buckets
                .range(now.saturating_sub(DAY) / MINUTE * MINUTE..=now)
                .map(|(&at, b)| MinuteBucketDto {
                    at_ms: at as f64,
                    cpu: b.cpu.average(),
                    cpu_min: b.cpu.min,
                    cpu_max: b.cpu.max,
                    memory: b.memory.average(),
                    cpu_coverage_ms: b.cpu.covered_ms as f64,
                    memory_coverage_ms: b.memory.covered_ms as f64,
                    download: (b.network_ms > 0)
                        .then(|| b.received as f64 * 1000. / b.network_ms as f64),
                    upload: (b.network_ms > 0)
                        .then(|| b.transmitted as f64 * 1000. / b.network_ms as f64),
                    network_coverage_ms: b.network_ms as f64,
                    network_key: if b.multiple_networks {
                        format!("mixed-{at}")
                    } else {
                        b.networks.first().map(|n| n.id.clone()).unwrap_or_default()
                    },
                })
                .collect(),
        }
    }
}
#[tauri::command]
pub fn get_archive_snapshot(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, Arc<crate::runtime::Runtime>>,
    day_start_ms: f64,
) -> Result<ArchiveSnapshotDto, String> {
    if window.label() != "main" {
        return Err("仅主窗口可以查看本地历史".into());
    }
    let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
    if !day_start_ms.is_finite()
        || day_start_ms < now.saturating_sub(27 * 60 * MINUTE) as f64
        || day_start_ms > now as f64
        || day_start_ms % MINUTE as f64 != 0.
    {
        return Err("本地日期边界无效".into());
    }
    runtime
        .archives
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.snapshot(day_start_ms as u64, now))
        .ok_or_else(|| "本地历史目录不可用".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_drains_queue_and_restart_preserves_exact_totals() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.json");
        let input = |at, duration, received| ArchiveInput {
            wall_ms: DAY + at,
            elapsed_ms: duration,
            cpu: Some(25.),
            memory: Some(50.),
            traffic: (duration > 0).then_some(pinmeter_core::domain::TrafficDelta {
                received,
                transmitted: 3,
                elapsed_ms: duration,
            }),
            network: Some(pinmeter_core::archive::NetworkSource {
                id: "eth".into(),
                name: "以太网".into(),
            }),
        };
        let service = Archives::start(path.clone());
        service.offer(input(1000, 0, 0));
        service.offer(input(2000, 1000, 11));
        service.stop();
        let loaded = pinmeter_platform::archive::ArchiveFile::new(path.clone())
            .load()
            .unwrap();
        assert_eq!(loaded.totals(DAY, DAY + MINUTE).received, 11);
        let service = Archives::start(path.clone());
        service.offer(input(10000, 0, 0));
        service.offer(input(11000, 1000, 17));
        service.stop();
        let loaded = pinmeter_platform::archive::ArchiveFile::new(path)
            .load()
            .unwrap();
        assert_eq!(loaded.totals(DAY, DAY + MINUTE).received, 28);
        assert_eq!(loaded.totals(DAY, DAY + MINUTE).covered_ms, 2000);
    }
}
