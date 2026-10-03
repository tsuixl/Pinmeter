use pinmeter_core::{
    app_history::{AppHistory, AppHistoryInput},
    archive::{DAY, MINUTE},
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
pub struct AppHistories {
    data: Mutex<Data>,
    tx: SyncSender<AppHistoryInput>,
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
                if let Ok(input) = rx.recv_timeout(Duration::from_millis(250)) {
                    dirty |= s.data.lock().unwrap().history.accept(input);
                }
                for input in rx.try_iter() {
                    dirty |= s.data.lock().unwrap().history.accept(input);
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
        if self.tx.try_send(input).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
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
        let summary = h.summarize(start, end.max(start));
        let identity = selected.as_deref().and_then(|id| h.identity(id));
        let points = if identity.is_some() {
            h.series(selected.as_deref().unwrap(), start, end)
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
        _ => return Err("未知应用历史范围".into()),
    }
    .max(now.saturating_sub(DAY))
        / MINUTE
        * MINUTE;
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
