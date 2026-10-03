use crate::{
    contracts::ReadingStatus,
    presenters,
    runtime::Runtime,
    sampling::{SamplingAction, SamplingWait},
};
use pinmeter_core::{domain::Status, ports::Clock, processes::ProcessRanking};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
};
use ts_rs::TS;
static NEXT_SERVICE_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Serialize, TS)]
pub struct ProcessRowDto {
    pub id: String,
    pub pid: u32,
    pub name: String,
    pub application_id: String,
    pub cpu: Option<f64>,
    pub cpu_status: ReadingStatus,
    pub working_set: Option<f64>,
    pub memory_status: ReadingStatus,
}
#[derive(Clone, Serialize, TS)]
pub struct ProcessApplicationDto {
    pub id: String,
    pub name: String,
    pub cpu: Option<f64>,
    pub cpu_status: ReadingStatus,
    pub working_set: Option<f64>,
    pub memory_status: ReadingStatus,
    pub process_count: u32,
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
    pub applications: Vec<ProcessApplicationDto>,
}
struct Data {
    ranking: ProcessRanking,
    status: Status,
    detail: String,
    wall: Option<u64>,
}
pub struct Processes {
    service_id: u64,
    data: Mutex<Data>,
    sampling: SamplingWait,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Processes {
    pub fn start(visible: bool) -> Arc<Self> {
        let service = Arc::new(Self {
            service_id: NEXT_SERVICE_ID.fetch_add(1, Ordering::Relaxed),
            data: Mutex::new(Data {
                ranking: ProcessRanking::default(),
                status: Status::Warming,
                detail: "正在建立进程 CPU 基线".into(),
                wall: None,
            }),
            sampling: SamplingWait::new(visible),
            worker: Mutex::new(None),
        });
        let s = service.clone();
        *service.worker.lock().unwrap() = Some(thread::spawn(move || {
            let clock = pinmeter_platform::shared::SystemClock::default();
            loop {
                let generation = match s.sampling.wait() {
                    SamplingAction::Stop => break,
                    SamplingAction::Reset => {
                        let mut data = s.data.lock().unwrap();
                        data.ranking.reset();
                        data.status = Status::Warming;
                        data.detail = "进程采集已暂停，打开页面后重新采样".into();
                        data.wall = None;
                        continue;
                    }
                    SamplingAction::Collect(generation) => generation,
                };
                let result = pinmeter_platform::processes::collect();
                s.sampling.finish(generation, || {
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
                });
            }
        }));
        service
    }
    pub fn stop(&self) {
        self.sampling.stop();
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
    pub fn set_visible(&self, visible: bool) {
        self.sampling.set_visible(visible);
    }
    pub fn diagnostic_snapshot(&self) -> ProcessSnapshotDto {
        self.snapshot(false, false)
    }
    pub fn diagnostic_until(&self, until: Option<std::time::Instant>) {
        self.sampling.diagnostic_until(until);
    }
    fn snapshot(&self, memory: bool, visible: bool) -> ProcessSnapshotDto {
        if visible {
            self.sampling.request();
        }
        let data = self.data.lock().unwrap();
        let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
        let stale = data
            .ranking
            .sampled_at
            .is_some_and(|at| now.saturating_sub(at) > 6_000);
        let rows: Vec<_> = if stale {
            vec![]
        } else {
            data.ranking
                .rows
                .iter()
                .map(|r| ProcessRowDto {
                    id: format!("{}:{}", self.service_id, r.identity),
                    pid: r.pid,
                    name: r.name.clone(),
                    application_id: format!("{}:{}", self.service_id, r.application_id),
                    cpu: r.cpu,
                    cpu_status: presenters::status(r.cpu_status),
                    working_set: r.working_set.map(|v| v as f64),
                    memory_status: presenters::status(r.memory_status),
                })
                .collect()
        };
        let warming = stale
            || (!memory
                && !rows.iter().any(|r| r.cpu.is_some())
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
            applications: if stale {
                vec![]
            } else {
                data.ranking
                    .applications()
                    .into_iter()
                    .map(|a| ProcessApplicationDto {
                        id: format!("{}:{}", self.service_id, a.id),
                        name: a.name,
                        cpu: a.cpu,
                        cpu_status: presenters::status(a.cpu_status),
                        working_set: a.working_set.map(|v| v as f64),
                        memory_status: presenters::status(a.memory_status),
                        process_count: a.process_count as u32,
                    })
                    .collect()
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use pinmeter_core::processes::{ProcessBatch, ProcessSample};
    #[test]
    fn restarted_service_cannot_reuse_a_previously_pinned_identity() {
        let first = Processes::start(false);
        let second = Processes::start(false);
        for service in [&first, &second] {
            service.data.lock().unwrap().ranking.accept(
                ProcessBatch {
                    rows: vec![ProcessSample {
                        pid: 42,
                        name: "same.exe".into(),
                        executable: Some("C:\\same.exe".into()),
                        times: Ok((10, 0)),
                        working_set: Ok(100),
                    }],
                    logical_cpus: 1,
                    truncated: false,
                },
                pinmeter_platform::shared::SystemClock::default().monotonic_ms(),
            );
        }
        let previous = first.snapshot(true, false);
        let restarted = second.snapshot(true, false);
        first.stop();
        second.stop();
        assert_ne!(previous.rows[0].id, restarted.rows[0].id);
        assert_ne!(previous.applications[0].id, restarted.applications[0].id);
        assert_eq!(
            restarted.rows[0].application_id,
            restarted.applications[0].id
        );
    }
}
