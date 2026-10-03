use pinmeter_core::{
    archive::{Archive, ArchiveInput, DAY, HOUR, MINUTE, MONTH, QUARTER_HOUR, WEEK, resolution},
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
pub(crate) mod management;
pub(crate) use management::csv_cell;
pub use management::{HistoryExportDto, HistoryStorageDto};
#[derive(Clone, Serialize, TS)]
pub struct MinuteBucketDto {
    pub at_ms: f64,
    pub cpu: Option<f64>,
    pub cpu_min: Option<f64>,
    pub cpu_max: Option<f64>,
    pub cpu_max_at_ms: Option<f64>,
    pub memory: Option<f64>,
    pub memory_min: Option<f64>,
    pub memory_max: Option<f64>,
    pub memory_max_at_ms: Option<f64>,
    pub cpu_coverage_ms: f64,
    pub memory_coverage_ms: f64,
    pub download: Option<f64>,
    pub upload: Option<f64>,
    pub download_max: Option<f64>,
    pub download_max_at_ms: Option<f64>,
    pub upload_max: Option<f64>,
    pub upload_max_at_ms: Option<f64>,
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
    pub from_ms: f64,
    pub resolution_ms: f64,
    pub current_period: PeriodSummaryDto,
    pub previous_period: Option<PeriodSummaryDto>,
    pub received: String,
    pub transmitted: String,
    pub network_coverage_ms: f64,
    pub networks: Vec<String>,
    pub buckets: Vec<MinuteBucketDto>,
}
#[derive(Clone, Serialize, TS)]
pub struct PeriodSummaryDto {
    pub from_ms: f64,
    pub through_ms: f64,
    pub cpu_average: Option<f64>,
    pub cpu_coverage_ms: f64,
    pub memory_average: Option<f64>,
    pub memory_coverage_ms: f64,
    pub received: String,
    pub transmitted: String,
    pub network_coverage_ms: f64,
}
struct Data {
    archive: Archive,
    loading: bool,
    notice: String,
    error: String,
    saved_at: Option<u64>,
}
enum ArchiveCommand {
    Sample(ArchiveInput),
    Clear {
        cutoff: u64,
        reply: mpsc::Sender<Result<(), String>>,
    },
}
pub struct Archives {
    tx: SyncSender<ArchiveCommand>,
    path: PathBuf,
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
            path: path.clone(),
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
                if let Ok(command) = rx.recv_timeout(Duration::from_millis(250)) {
                    s.apply_command(command, &store, &mut writable, &mut dirty);
                }
                for command in rx.try_iter() {
                    s.apply_command(command, &store, &mut writable, &mut dirty);
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
        if self.tx.try_send(ArchiveCommand::Sample(input)).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn apply_command(
        &self,
        command: ArchiveCommand,
        store: &pinmeter_platform::archive::ArchiveFile,
        writable: &mut bool,
        dirty: &mut bool,
    ) {
        match command {
            ArchiveCommand::Sample(input) => {
                self.data.lock().unwrap().archive.accept(input);
                *dirty = true;
            }
            ArchiveCommand::Clear { cutoff, reply } => {
                let empty = Archive::cleared(cutoff);
                let result = store.clear(&empty);
                let mut data = self.data.lock().unwrap();
                if result.is_ok() {
                    data.archive = empty;
                    data.error.clear();
                    data.notice = "基础历史已清除；继续采集时会产生新的记录。".into();
                    data.saved_at = Some(cutoff);
                    self.lost.store(0, Ordering::Relaxed);
                    *writable = true;
                    *dirty = false;
                } else if let Err(error) = &result {
                    data.error = error.clone();
                }
                let _ = reply.send(result);
            }
        }
    }
    pub fn clear(&self, cutoff: u64) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) {
            return Err("历史服务正在退出".into());
        }
        let (reply, result) = mpsc::channel();
        self.tx
            .send(ArchiveCommand::Clear { cutoff, reply })
            .map_err(|_| "历史服务已停止")?;
        result.recv().map_err(|_| "历史清除未完成".to_string())?
    }
    pub fn storage_bytes(&self) -> Result<u64, String> {
        pinmeter_platform::archive::ArchiveFile::new(self.path.clone()).storage_bytes()
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
    fn snapshot(&self, day_start: u64, now: u64, range_ms: u64) -> ArchiveSnapshotDto {
        let data = self.data.lock().unwrap();
        let a = &data.archive;
        let total = a.totals(day_start, now.max(day_start));
        let resolution_ms = resolution(range_ms);
        let from = now.saturating_sub(range_ms) / resolution_ms * resolution_ms;
        let comparison_resolution = if range_ms <= 6 * HOUR {
            MINUTE
        } else if range_ms <= DAY {
            QUARTER_HOUR
        } else {
            HOUR
        };
        let comparison_end = now / comparison_resolution * comparison_resolution;
        let comparison_start = comparison_end.saturating_sub(range_ms);
        ArchiveSnapshotDto {
            loading: data.loading,
            notice: data.notice.clone(),
            persistence_error: data.error.clone(),
            saved_at_ms: data.saved_at.map(|v| v as f64),
            lost_samples: self.lost.load(Ordering::Relaxed).to_string(),
            clock_discontinuities: a.clock_discontinuities.to_string(),
            now_ms: now as f64,
            day_start_ms: day_start as f64,
            from_ms: from as f64,
            resolution_ms: resolution_ms as f64,
            current_period: period(a, comparison_start, comparison_end, comparison_resolution),
            previous_period: (range_ms < MONTH).then(|| {
                period(
                    a,
                    comparison_start.saturating_sub(range_ms),
                    comparison_start,
                    comparison_resolution,
                )
            }),
            received: total.received.to_string(),
            transmitted: total.transmitted.to_string(),
            network_coverage_ms: total.covered_ms as f64,
            networks: total.networks,
            buckets: a
                .layer(resolution_ms)
                .range(from..=now)
                .map(|(&at, b)| MinuteBucketDto {
                    at_ms: at as f64,
                    cpu: b.cpu.average(),
                    cpu_min: b.cpu.min,
                    cpu_max: b.cpu.max,
                    cpu_max_at_ms: b.cpu.max_at_ms.map(|at| at as f64),
                    memory: b.memory.average(),
                    memory_min: b.memory.min,
                    memory_max: b.memory.max,
                    memory_max_at_ms: b.memory.max_at_ms.map(|at| at as f64),
                    cpu_coverage_ms: b.cpu.covered_ms as f64,
                    memory_coverage_ms: b.memory.covered_ms as f64,
                    download: (b.network_ms > 0)
                        .then(|| b.received as f64 * 1000. / b.network_ms as f64),
                    upload: (b.network_ms > 0)
                        .then(|| b.transmitted as f64 * 1000. / b.network_ms as f64),
                    download_max: b.download_peak.as_ref().map(|peak| peak.value),
                    download_max_at_ms: b.download_peak.as_ref().map(|peak| peak.at_ms as f64),
                    upload_max: b.upload_peak.as_ref().map(|peak| peak.value),
                    upload_max_at_ms: b.upload_peak.as_ref().map(|peak| peak.at_ms as f64),
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
    range_ms: Option<f64>,
) -> Result<ArchiveSnapshotDto, String> {
    if window.label() != "main" {
        return Err("仅主窗口可以查看本地历史".into());
    }
    let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
    let range_ms = validated_range(range_ms.unwrap_or(DAY as f64))?;
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
        .map(|s| s.snapshot(day_start_ms as u64, now, range_ms))
        .ok_or_else(|| "本地历史目录不可用".into())
}

fn validated_range(range_ms: f64) -> Result<u64, String> {
    [HOUR, 6 * HOUR, DAY, WEEK, MONTH]
        .into_iter()
        .find(|value| *value as f64 == range_ms)
        .ok_or_else(|| "未知历史时间范围".into())
}
fn period(archive: &Archive, from: u64, through: u64, resolution_ms: u64) -> PeriodSummaryDto {
    let (mut cpu_sum, mut memory_sum) = (0., 0.);
    let (mut cpu_ms, mut memory_ms, mut network_ms) = (0u64, 0u64, 0u64);
    let (mut received, mut transmitted) = (0u128, 0u128);
    for bucket in archive
        .layer(resolution_ms)
        .range(from..through)
        .map(|(_, bucket)| bucket)
    {
        cpu_sum += bucket.cpu.weighted_sum;
        cpu_ms += bucket.cpu.covered_ms;
        memory_sum += bucket.memory.weighted_sum;
        memory_ms += bucket.memory.covered_ms;
        network_ms += bucket.network_ms;
        received += bucket.received as u128;
        transmitted += bucket.transmitted as u128;
    }
    PeriodSummaryDto {
        from_ms: from as f64,
        through_ms: through as f64,
        cpu_average: (cpu_ms > 0).then(|| cpu_sum / cpu_ms as f64),
        cpu_coverage_ms: cpu_ms as f64,
        memory_average: (memory_ms > 0).then(|| memory_sum / memory_ms as f64),
        memory_coverage_ms: memory_ms as f64,
        received: received.to_string(),
        transmitted: transmitted.to_string(),
        network_coverage_ms: network_ms as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_acknowledges_persistence_and_old_queued_samples_cannot_reappear() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.json");
        let service = Archives::start(path.clone());
        let sample = |wall_ms| ArchiveInput {
            wall_ms,
            elapsed_ms: 1000,
            cpu: Some(20.),
            memory: Some(40.),
            traffic: None,
            network: None,
        };
        service.offer(sample(DAY + 1000));
        service.clear(DAY + 2000).unwrap();
        assert!(
            pinmeter_platform::archive::ArchiveFile::new(path.clone())
                .load()
                .unwrap()
                .buckets
                .is_empty()
        );
        service.offer(sample(DAY + 1500));
        service.offer(sample(DAY + 2500));
        service.offer(sample(DAY + 3500));
        service.stop();
        let restored = pinmeter_platform::archive::ArchiveFile::new(path)
            .load()
            .unwrap();
        assert_eq!(restored.buckets[&DAY].cpu.covered_ms, 1000);
        assert_eq!(restored.cleared_before_ms, DAY + 2000);
    }
    #[test]
    fn a_failed_clear_does_not_claim_success_or_discard_in_memory_history() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("not-a-directory");
        std::fs::write(&parent, "keep").unwrap();
        let service = Archives::start(parent.join("history.json"));
        service.offer(ArchiveInput {
            wall_ms: DAY + 1000,
            elapsed_ms: 1000,
            cpu: Some(20.),
            memory: None,
            traffic: None,
            network: None,
        });
        assert!(service.clear(DAY + 2000).is_err());
        assert_eq!(service.data.lock().unwrap().archive.buckets.len(), 1);
        service.stop();
    }
    #[test]
    fn comparison_uses_raw_weights_and_same_length_periods() {
        let mut archive = Archive::default();
        for (wall_ms, elapsed_ms, cpu) in [(DAY + 1000, 1000, 100.), (DAY + 10000, 9000, 0.)] {
            archive.accept(ArchiveInput {
                wall_ms,
                elapsed_ms,
                cpu: Some(cpu),
                memory: None,
                traffic: None,
                network: None,
            });
        }
        let summary = period(&archive, DAY, 2 * DAY, QUARTER_HOUR);
        assert_eq!(summary.cpu_average, Some(10.));
        assert_eq!(summary.cpu_coverage_ms, 10000.);
        assert_eq!(summary.through_ms - summary.from_ms, DAY as f64);
    }
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
