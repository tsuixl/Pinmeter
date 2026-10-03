use crate::{diagnostics::DiagnosticExportDto, runtime::Runtime};
use pinmeter_core::{ports::Clock, trace::*};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{Manager, State, WebviewWindow};
use ts_rs::TS;

#[derive(Clone, Serialize, TS)]
pub struct TraceSnapshotDto {
    pub token: String,
    pub active: bool,
    pub remaining_seconds: u32,
    pub samples: u32,
    pub content: String,
}
#[derive(Default)]
struct Data {
    generation: u64,
    deadline: Option<Instant>,
    report: Option<TraceReport>,
    processes: Option<Arc<crate::processes::Processes>>,
}
#[derive(Default)]
pub struct Traces {
    data: Mutex<Data>,
}
impl Traces {
    fn snapshot(&self) -> TraceSnapshotDto {
        let data = self.data.lock().unwrap();
        let active = data.deadline.is_some();
        TraceSnapshotDto {
            token: data.generation.to_string(),
            active,
            remaining_seconds: data.deadline.map_or(0, |t| {
                t.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(120) as u32
            }),
            samples: data
                .report
                .as_ref()
                .map_or(0, |r| r.observations.len() as u32),
            content: data.report.as_ref().map_or(String::new(), |r| {
                serde_json::to_string_pretty(r).unwrap_or_default()
            }),
        }
    }
    fn finish(&self, generation: Option<u64>) {
        let mut data = self.data.lock().unwrap();
        if generation.is_some_and(|g| g != data.generation) {
            return;
        }
        data.deadline = None;
        if let Some(report) = data.report.as_mut() {
            report
                .finished_at_ms
                .get_or_insert(crate::updates::now_ms());
        }
        if let Some(processes) = data.processes.take() {
            processes.diagnostic_until(None);
        }
    }
}
fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("仅主窗口可以管理排障记录".into())
    }
}
#[tauri::command]
pub fn get_trace_snapshot(
    window: WebviewWindow,
    traces: State<'_, Arc<Traces>>,
) -> Result<TraceSnapshotDto, String> {
    authorize(&window)?;
    Ok(traces.snapshot())
}
#[tauri::command]
pub fn start_trace(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    traces: State<'_, Arc<Traces>>,
    seconds: u32,
) -> Result<TraceSnapshotDto, String> {
    authorize(&window)?;
    if runtime.exit.lock().map_err(|e| e.to_string())?.stage != "idle" {
        return Err("正在退出或安装更新，请稍后开始排障".into());
    }
    let started = crate::updates::now_ms();
    let before = {
        let state = runtime.inner.lock().map_err(|e| e.to_string())?;
        let mut last = u64::MAX;
        let mut points = Vec::new();
        for frame in state.monitor.history.iter().rev() {
            if frame.at_ms < started.saturating_sub(60_000) || points.len() == 12 {
                break;
            }
            if last.saturating_sub(frame.at_ms) >= 5_000 {
                points.push(TracePoint::from(frame));
                last = frame.at_ms;
            }
        }
        points.reverse();
        points
    };
    let report = TraceReport::new(started, seconds, before)?;
    let mut data = traces.data.lock().map_err(|e| e.to_string())?;
    if data.deadline.is_some() {
        return Err("已有排障记录正在进行".into());
    }
    let processes = runtime
        .processes
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("进程服务正在启动")?;
    let deadline = Instant::now() + Duration::from_secs(u64::from(seconds));
    data.generation += 1;
    let generation = data.generation;
    data.report = Some(report);
    data.deadline = Some(deadline);
    data.processes = Some(processes.clone());
    processes.diagnostic_until(Some(deadline));
    drop(data);
    let service = traces.inner().clone();
    let owner = Arc::downgrade(runtime.inner());
    let spawned = thread::Builder::new()
        .name("pinmeter-trace".into())
        .spawn(move || {
            loop {
                let Some(runtime) = owner.upgrade() else {
                    service.finish(Some(generation));
                    break;
                };
                let still_active = {
                    let d = service.data.lock().unwrap();
                    d.generation == generation && d.deadline.is_some()
                };
                if !still_active {
                    break;
                }
                let same_service = || {
                    runtime
                        .processes
                        .lock()
                        .unwrap()
                        .as_ref()
                        .is_some_and(|current| Arc::ptr_eq(current, &processes))
                };
                if Instant::now() >= deadline || runtime.is_stopped() || !same_service() {
                    service.finish(Some(generation));
                    break;
                }
                let now = crate::updates::now_ms();
                let mono = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
                let metrics = runtime
                    .inner
                    .lock()
                    .unwrap()
                    .monitor
                    .latest_at(mono)
                    .as_ref()
                    .map(TracePoint::from);
                let process = processes.diagnostic_snapshot();
                if !same_service() {
                    service.finish(Some(generation));
                    break;
                }
                let mut data = service.data.lock().unwrap();
                if data.generation != generation || data.deadline.is_none() {
                    break;
                }
                if let Some(report) = data.report.as_mut() {
                    report.record(TraceObservation {
                        observed_at_ms: now,
                        metrics,
                        processes_at_ms: process.sampled_at_ms.map(|v| v as u64),
                        processes_status: serde_json::to_value(process.status)
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_string))
                            .unwrap_or_else(|| "failed".into()),
                        top_cpu_processes: process
                            .rows
                            .into_iter()
                            .filter(|p| {
                                matches!(p.cpu_status, crate::contracts::ReadingStatus::Normal)
                            })
                            .filter_map(|p| {
                                p.cpu.map(|cpu| TraceProcess {
                                    pid: p.pid,
                                    name: p.name,
                                    cpu,
                                })
                            })
                            .collect(),
                    });
                }
                drop(data);
                // Bound stop latency without holding a sampling or UI lock.
                thread::sleep(Duration::from_secs(2));
            }
        });
    if let Err(error) = spawned {
        traces.finish(Some(generation));
        return Err(format!("无法开始排障：{error}"));
    }
    Ok(traces.snapshot())
}
#[tauri::command]
pub fn stop_trace(
    window: WebviewWindow,
    traces: State<'_, Arc<Traces>>,
) -> Result<TraceSnapshotDto, String> {
    authorize(&window)?;
    traces.finish(None);
    Ok(traces.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_old_worker_cannot_finish_a_new_trace_or_release_its_lease() {
        let traces = Traces::default();
        {
            let mut data = traces.data.lock().unwrap();
            data.generation = 2;
            data.deadline = Some(Instant::now() + Duration::from_secs(30));
            data.report = Some(TraceReport::new(100, 30, vec![]).unwrap());
        }
        traces.finish(Some(1));
        assert!(traces.snapshot().active);
        assert!(
            traces
                .data
                .lock()
                .unwrap()
                .report
                .as_ref()
                .unwrap()
                .finished_at_ms
                .is_none()
        );
        traces.finish(Some(2));
        assert!(!traces.snapshot().active);
        assert!(
            traces
                .data
                .lock()
                .unwrap()
                .report
                .as_ref()
                .unwrap()
                .finished_at_ms
                .is_some()
        );
    }
}
#[tauri::command]
pub async fn export_trace(
    window: WebviewWindow,
    traces: State<'_, Arc<Traces>>,
    token: String,
) -> Result<DiagnosticExportDto, String> {
    authorize(&window)?;
    let preview = traces.snapshot();
    if preview.active || preview.content.is_empty() || preview.token != token {
        return Err("请结束记录并重新预览后导出".into());
    }
    let directory = window
        .app_handle()
        .path()
        .download_dir()
        .map_err(|_| "下载目录不可用")?;
    let path = tauri::async_runtime::spawn_blocking(move || {
        pinmeter_platform::diagnostics::export_trace(
            &directory,
            &preview.content,
            crate::updates::now_ms(),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(DiagnosticExportDto {
        saved: true,
        path: Some(path.to_string_lossy().into_owned()),
    })
}
