use pinmeter_core::{
    app_history::{AppHistory, AppHistoryInput},
    archive::{DAY, MINUTE, MONTH, WEEK, resolution},
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
pub struct AppHistoryRowDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub received: String,
    pub transmitted: String,
    pub total: String,
}
#[derive(Clone, Serialize, TS)]
pub struct AppHistoryPointDto {
    pub at_ms: f64,
    pub download: Option<f64>,
    pub upload: Option<f64>,
}
#[derive(Clone, Serialize, TS)]
pub struct AppHistorySnapshotDto {
    pub loading: bool,
    pub notice: String,
    pub persistence_error: String,
    pub saved_at_ms: Option<f64>,
    pub lost_windows: String,
    pub clock_discontinuities: String,
    pub range: String,
    pub from_ms: f64,
    pub through_ms: f64,
    pub resolution_ms: f64,
    pub received: String,
    pub transmitted: String,
    pub covered_ms: f64,
    pub observed_ms: f64,
    pub incomplete: bool,
    pub limited: bool,
    pub skipped_windows: String,
    pub rows: Vec<AppHistoryRowDto>,
    pub selected_id: Option<String>,
    pub selected_name: Option<String>,
    pub points: Vec<AppHistoryPointDto>,
}
struct Data {
    history: AppHistory,
    loading: bool,
    notice: String,
    error: String,
    saved: Option<u64>,
}
enum AppHistoryCommand {
    Sample(AppHistoryInput),
    Clear {
        cutoff: u64,
        reply: mpsc::Sender<Result<(), String>>,
    },
}
pub struct AppHistories {
    data: Mutex<Data>,
    tx: SyncSender<AppHistoryCommand>,
    path: PathBuf,
    stop: AtomicBool,
    lost: AtomicU64,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl AppHistories {
    pub fn start(path: PathBuf) -> Arc<Self> {
        let (tx, rx) = mpsc::sync_channel(64);
        let service = Arc::new(Self {
            data: Mutex::new(Data {
                history: AppHistory::default(),
                loading: true,
                notice: String::new(),
                error: String::new(),
                saved: None,
            }),
            tx,
            path: path.clone(),
            stop: AtomicBool::new(false),
            lost: AtomicU64::new(0),
            worker: Mutex::new(None),
        });
        let s = service.clone();
        *service.worker.lock().unwrap() = Some(thread::spawn(move || {
            let file = pinmeter_platform::archive::AppHistoryFile::new(path);
            let clock = pinmeter_platform::shared::SystemClock::default();
            let mut writable = true;
            // Loading and file I/O never hold the live-monitor mutex.
            let loaded = file.load();
            {
                let mut data = s.data.lock().unwrap();
                match loaded {
                    Ok(history) => {
                        data.saved = (history.last_wall_ms > 0).then_some(history.last_wall_ms);
                        data.history = history;
                    }
                    Err(error) => match file.preserve_invalid() {
                        Ok(()) => data.notice = format!("{error}；原件已备份，从本次重新记录。"),
                        Err(reason) => {
                            writable = false;
                            data.error = format!("{error}；{reason}");
                        }
                    },
                }
                data.loading = false;
            }
            let mut next = Instant::now() + Duration::from_secs(60);
            let mut dirty = s.data.lock().unwrap().history.prune(clock.wall_ms());
            loop {
                if let Ok(command) = rx.recv_timeout(Duration::from_millis(250)) {
                    s.apply_command(command, &file, &mut writable, &mut dirty);
                }
                for command in rx.try_iter() {
                    s.apply_command(command, &file, &mut writable, &mut dirty);
                }
                let stopping = s.stop.load(Ordering::Acquire);
                if stopping || Instant::now() >= next {
                    dirty |= s.data.lock().unwrap().history.prune(clock.wall_ms());
                    if dirty && writable {
                        let copy = s.data.lock().unwrap().history.clone();
                        let result = file.save(&copy);
                        let mut data = s.data.lock().unwrap();
                        match result {
                            Ok(()) => {
                                data.error.clear();
                                data.saved = Some(copy.last_wall_ms);
                                dirty = false;
                            }
                            Err(error) => data.error = error,
                        }
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
    pub fn offer(&self, input: AppHistoryInput) {
        if self.tx.try_send(AppHistoryCommand::Sample(input)).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn apply_command(
        &self,
        command: AppHistoryCommand,
        file: &pinmeter_platform::archive::AppHistoryFile,
        writable: &mut bool,
        dirty: &mut bool,
    ) {
        match command {
            AppHistoryCommand::Sample(input) => {
                *dirty |= self.data.lock().unwrap().history.accept(input)
            }
            AppHistoryCommand::Clear { cutoff, reply } => {
                let empty = AppHistory::cleared(cutoff);
                let result = file.clear(&empty);
                let mut data = self.data.lock().unwrap();
                if result.is_ok() {
                    data.history = empty;
                    data.error.clear();
                    data.notice = "应用历史已清除；开启记录时会产生新的记录。".into();
                    data.saved = Some(cutoff);
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
            return Err("应用历史服务正在退出".into());
        }
        let (reply, result) = mpsc::channel();
        self.tx
            .send(AppHistoryCommand::Clear { cutoff, reply })
            .map_err(|_| "应用历史服务已停止")?;
        result
            .recv()
            .map_err(|_| "应用历史清除未完成".to_string())?
    }
    pub fn storage_bytes(&self) -> Result<u64, String> {
        pinmeter_platform::archive::AppHistoryFile::new(self.path.clone()).storage_bytes()
    }
    pub(crate) fn export_rows(
        &self,
        from: u64,
        through: u64,
        resolution_ms: u64,
        include_paths: bool,
    ) -> Result<String, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let history = &data.history;
        let mut csv = "bucket_start_unix_ms,resolution_ms,application,path,received_bytes,transmitted_bytes,observed_ms,covered_ms,incomplete\r\n".to_string();
        for (&at, bucket) in history
            .layer(resolution_ms)
            .range(from / resolution_ms * resolution_ms..through)
        {
            let mut rows: Vec<(String, String, [u64; 2])> = bucket
                .apps
                .iter()
                .filter_map(|(id, bytes)| {
                    history.apps.get(id).map(|app| {
                        (
                            app.name.clone(),
                            if include_paths {
                                app.path.clone()
                            } else {
                                String::new()
                            },
                            *bytes,
                        )
                    })
                })
                .collect();
            for (label, bytes) in [("未归属", bucket.unknown), ("其他已归属", bucket.other)]
            {
                if bytes != [0, 0] {
                    rows.push((label.into(), String::new(), bytes));
                }
            }
            if rows.is_empty() {
                rows.push(("无已记录流量".into(), String::new(), [0, 0]));
            }
            for (name, path, bytes) in rows {
                let observed = bucket.observed_ms > 0;
                csv.push_str(
                    &[
                        at.to_string(),
                        resolution_ms.to_string(),
                        crate::archive::csv_cell(&name),
                        crate::archive::csv_cell(&path),
                        if observed {
                            bytes[0].to_string()
                        } else {
                            String::new()
                        },
                        if observed {
                            bytes[1].to_string()
                        } else {
                            String::new()
                        },
                        bucket.observed_ms.to_string(),
                        bucket.covered_ms.to_string(),
                        bucket.incomplete.to_string(),
                    ]
                    .join(","),
                );
                csv.push_str("\r\n");
                if csv.len() > 32 * 1024 * 1024 {
                    return Err("导出超过32 MiB，请缩短时间范围或关闭完整路径".into());
                }
            }
        }
        Ok(csv)
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.lock().unwrap().take() {
            let _ = w.join();
        }
    }
    fn snapshot(
        &self,
        range: String,
        start: u64,
        end: u64,
        selected: Option<String>,
    ) -> AppHistorySnapshotDto {
        let data = self.data.lock().unwrap();
        let h = &data.history;
        let resolution_ms = match range.as_str() {
            "7d" => resolution(WEEK),
            "30d" => resolution(MONTH),
            _ => MINUTE,
        };
        let summary = h.summarize_at(start, end.max(start), resolution_ms);
        let identity = selected.as_deref().and_then(|id| h.identity(id));
        let points = if identity.is_some() {
            h.series_at(selected.as_deref().unwrap(), start, end, resolution_ms)
                .into_iter()
                .map(|p| AppHistoryPointDto {
                    at_ms: p.at_ms as f64,
                    download: p.download,
                    upload: p.upload,
                })
                .collect()
        } else {
            vec![]
        };
        AppHistorySnapshotDto {
            loading: data.loading,
            notice: data.notice.clone(),
            persistence_error: data.error.clone(),
            saved_at_ms: data.saved.map(|v| v as f64),
            lost_windows: self.lost.load(Ordering::Relaxed).to_string(),
            clock_discontinuities: h.clock_discontinuities.to_string(),
            range,
            from_ms: start as f64,
            through_ms: end as f64,
            resolution_ms: resolution_ms as f64,
            received: summary.bytes[0].to_string(),
            transmitted: summary.bytes[1].to_string(),
            covered_ms: summary.covered_ms as f64,
            observed_ms: summary.observed_ms as f64,
            incomplete: summary.incomplete,
            limited: summary.limited,
            skipped_windows: summary.skipped.to_string(),
            rows: summary
                .rows
                .into_iter()
                .map(|r| AppHistoryRowDto {
                    id: r.id,
                    name: r.name,
                    path: r.path,
                    received: r.bytes[0].to_string(),
                    transmitted: r.bytes[1].to_string(),
                    total: (r.bytes[0] + r.bytes[1]).to_string(),
                })
                .collect(),
            selected_id: selected,
            selected_name: identity.map(|i| i.name),
            points,
        }
    }
}
#[tauri::command]
pub async fn get_app_history(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, Arc<crate::runtime::Runtime>>,
    range: String,
    day_start_ms: f64,
    app_id: Option<String>,
) -> Result<AppHistorySnapshotDto, String> {
    crate::commands::authorize(&window)?;
    let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
    if app_id.as_ref().is_some_and(|id| id.len() > 4100) {
        return Err("应用标识过长".into());
    }
    let start = match range.as_str() {
        "today" => {
            if !day_start_ms.is_finite()
                || day_start_ms < now.saturating_sub(DAY + 3 * 60 * MINUTE) as f64
                || day_start_ms > now as f64
                || day_start_ms % MINUTE as f64 != 0.
            {
                return Err("本地日期边界无效".into());
            }
            day_start_ms as u64
        }
        "1h" => now.saturating_sub(60 * MINUTE),
        "6h" => now.saturating_sub(6 * 60 * MINUTE),
        "24h" => now.saturating_sub(DAY),
        "7d" => now.saturating_sub(WEEK),
        "30d" => now.saturating_sub(MONTH),
        _ => return Err("未知应用历史范围".into()),
    }
    .max(now.saturating_sub(MONTH));
    let resolution_ms = match range.as_str() {
        "7d" => resolution(WEEK),
        "30d" => resolution(MONTH),
        _ => MINUTE,
    };
    let start = start / resolution_ms * resolution_ms;
    let service = runtime
        .app_history
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("应用历史目录不可用")?;
    tauri::async_runtime::spawn_blocking(move || service.snapshot(range, start, now, app_id))
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinmeter_core::app_history::AppWindow;
    #[test]
    fn clearing_app_history_is_persisted_before_ack_and_late_windows_stay_cleared() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("app-history.json");
        let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
        let input = |sequence, at, amount| AppHistoryInput {
            session: "clear-test".into(),
            wall_ms: at,
            window: AppWindow {
                generation: 1,
                sequence,
                elapsed_ms: 1000,
                complete: true,
                apps: std::collections::BTreeMap::from([("c:\\app.exe".into(), [amount, 0])]),
                unknown: [0, 0],
                other: [0, 0],
            },
        };
        let service = AppHistories::start(path.clone());
        service.offer(input(1, now, 50));
        service.clear(now + 1000).unwrap();
        assert!(
            pinmeter_platform::archive::AppHistoryFile::new(path.clone())
                .load()
                .unwrap()
                .apps
                .is_empty()
        );
        service.offer(input(2, now + 500, 50));
        service.offer(input(3, now + 1500, 50));
        service.offer(input(4, now + 2500, 7));
        service.stop();
        let restored = pinmeter_platform::archive::AppHistoryFile::new(path)
            .load()
            .unwrap();
        assert_eq!(
            restored
                .summarize(now.saturating_sub(MINUTE), now + MINUTE)
                .bytes,
            [7, 0]
        );
        assert_eq!(restored.cleared_before_ms, now + 1000);
    }
    #[test]
    fn final_queue_is_saved_and_next_session_adds_only_new_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("app-history.json");
        let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
        let input = |session: &str, sequence, at, amount| AppHistoryInput {
            session: session.into(),
            wall_ms: at,
            window: AppWindow {
                generation: 1,
                sequence,
                elapsed_ms: 1000,
                complete: true,
                apps: std::collections::BTreeMap::from([("c:\\app.exe".into(), [amount, 1])]),
                unknown: [0, 0],
                other: [0, 0],
            },
        };
        let service = AppHistories::start(path.clone());
        service.offer(input("run1", 1, now, 10));
        service.offer(input("run1", 2, now + 1000, 20));
        service.stop();
        let restored = pinmeter_platform::archive::AppHistoryFile::new(path.clone())
            .load()
            .unwrap();
        assert_eq!(
            restored
                .summarize(now.saturating_sub(DAY), now + MINUTE)
                .bytes,
            [30, 2]
        );
        let service = AppHistories::start(path.clone());
        service.offer(input("run2", 1, now + 2000, 5));
        service.stop();
        let restored = pinmeter_platform::archive::AppHistoryFile::new(path)
            .load()
            .unwrap();
        assert_eq!(
            restored
                .summarize(now.saturating_sub(DAY), now + MINUTE)
                .bytes,
            [35, 3]
        );
    }
}
