use crate::{contracts::ReadingStatus, presenters, runtime::Runtime};
use pinmeter_core::{domain::Status, ports::Clock, processes::ProcessRanking};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use ts_rs::TS;
#[derive(Clone, Serialize, TS)]
pub struct ProcessRowDto {
    pub id: String,
    pub pid: u32,
    pub name: String,
    pub cpu: Option<f64>,
    pub cpu_status: ReadingStatus,
    pub working_set: Option<f64>,
    pub memory_status: ReadingStatus,
}
#[derive(Clone, Serialize, TS)]
pub struct ProcessSnapshotDto {
    pub sort: String,
    pub status: ReadingStatus,
    pub detail: String,
    pub sampled_at_ms: Option<f64>,
    pub total: u32,
    pub unreadable: u32,
    pub truncated: bool,
    pub rows: Vec<ProcessRowDto>,
}
struct Data {
    ranking: ProcessRanking,
    requested: Option<Instant>,
    status: Status,
    detail: String,
    wall: Option<u64>,
}
pub struct Processes {
    data: Mutex<Data>,
    stop: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Processes {
    pub fn start(visible: Arc<AtomicBool>) -> Arc<Self> {
        let service = Arc::new(Self {
            data: Mutex::new(Data {
                ranking: ProcessRanking::default(),
                requested: None,
                status: Status::Warming,
                detail: "正在建立进程 CPU 基线".into(),
                wall: None,
            }),
            stop: AtomicBool::new(false),
            worker: Mutex::new(None),
        });
        let s = service.clone();
        *service.worker.lock().unwrap() = Some(thread::spawn(move || {
            let clock = pinmeter_platform::shared::SystemClock::default();
            let mut next = Instant::now();
            let mut active = false;
            while !s.stop.load(Ordering::Acquire) {
                let wanted = visible.load(Ordering::Acquire)
                    && s.data
                        .lock()
                        .unwrap()
                        .requested
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(5));
                if !wanted {
                    if active {
                        let mut data = s.data.lock().unwrap();
                        data.ranking.reset();
                        data.status = Status::Warming;
                        data.detail = "进程采集已暂停，打开页面后重新采样".into();
                        data.wall = None;
                    }
                    active = false;
                    next = Instant::now();
                } else if Instant::now() >= next {
                    active = true;
                    let result = pinmeter_platform::processes::collect();
                    let mut data = s.data.lock().unwrap();
                    match result {
                        Ok(batch) => {
                            data.ranking.accept(batch, clock.monotonic_ms());
                            data.status = Status::Normal;
                            data.detail = "按全机 CPU 百分比或内存工作集降序".into();
                            data.wall = Some(clock.wall_ms());
                        }
                        Err(e) => {
                            data.ranking.reset();
                            data.status = e.status;
                            data.detail = e.reason;
                            data.wall = None;
                        }
                    }
                    next = Instant::now() + Duration::from_secs(2);
                }
                thread::park_timeout(Duration::from_millis(200));
            }
        }));
        service
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
    fn snapshot(&self, memory: bool, visible: bool) -> ProcessSnapshotDto {
        let mut data = self.data.lock().unwrap();
        if visible {
            data.requested = Some(Instant::now());
        }
        let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
        let stale = data
            .ranking
            .sampled_at
            .is_some_and(|at| now.saturating_sub(at) > 6_000);
        let rows: Vec<_> = if stale {
            vec![]
        } else {
            data.ranking
                .top(memory)
                .into_iter()
                .map(|r| ProcessRowDto {
                    id: r.identity.clone(),
                    pid: r.pid,
                    name: r.name.clone(),
                    cpu: r.cpu,
                    cpu_status: presenters::status(r.cpu_status),
                    working_set: r.working_set.map(|v| v as f64),
                    memory_status: presenters::status(r.memory_status),
                })
                .collect()
        };
        let warming = stale
            || (!memory
                && rows.is_empty()
                && data
                    .ranking
                    .rows
                    .iter()
                    .any(|r| r.cpu_status == Status::Warming));
        ProcessSnapshotDto {
            sort: if memory { "memory" } else { "cpu" }.into(),
            status: presenters::status(if warming {
                Status::Warming
            } else {
                data.status
            }),
            detail: if warming {
                "需要两次有效采样计算进程 CPU".into()
            } else {
                data.detail.clone()
            },
            sampled_at_ms: data.wall.map(|v| v as f64),
            total: data.ranking.rows.len() as u32,
            unreadable: data.ranking.unreadable as u32,
            truncated: data.ranking.truncated,
            rows,
        }
    }
}
#[tauri::command]
pub fn get_process_snapshot(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, Arc<Runtime>>,
    sort: String,
) -> Result<ProcessSnapshotDto, String> {
    if window.label() != "main" {
        return Err("仅主窗口可以查看进程".into());
    }
    if !matches!(sort.as_str(), "cpu" | "memory") {
        return Err("未知排行字段".into());
    }
    runtime
        .processes
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.snapshot(sort == "memory", runtime.visible.load(Ordering::Acquire)))
        .ok_or_else(|| "进程服务正在启动".into())
}
