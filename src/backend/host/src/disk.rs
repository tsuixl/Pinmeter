use crate::{
    contracts::{ReadingDto, ReadingStatus},
    presenters,
    runtime::Runtime,
    sampling::{SamplingAction, SamplingWait},
};
use pinmeter_core::{disk::DiskMonitor, domain::Status, ports::Clock};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex, atomic::Ordering},
    thread::{self, JoinHandle},
};
use ts_rs::TS;

#[derive(Clone, Serialize, TS)]
pub struct DiskPointDto {
    pub id: String,
    pub read: ReadingDto,
    pub write: ReadingDto,
    pub activity: ReadingDto,
}
#[derive(Clone, Serialize, TS)]
pub struct DiskFrameDto {
    pub at_ms: f64,
    pub elapsed_ms: f64,
    pub generation: String,
    pub point: Option<DiskPointDto>,
}
#[derive(Clone, Serialize, TS)]
pub struct DiskSnapshotDto {
    pub status: ReadingStatus,
    pub detail: String,
    pub devices: Vec<DiskPointDto>,
    pub selected_id: Option<String>,
    pub history: Vec<DiskFrameDto>,
}
struct Data {
    monitor: DiskMonitor,
}
pub struct Disks {
    data: Mutex<Data>,
    sampling: SamplingWait,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Disks {
    pub fn start(visible: bool) -> Arc<Self> {
        let service = Arc::new(Self {
            data: Mutex::new(Data {
                monitor: DiskMonitor::default(),
            }),
            sampling: SamplingWait::new(visible),
            worker: Mutex::new(None),
        });
        let s = service.clone();
        *service.worker.lock().unwrap() = Some(thread::spawn(move || {
            let clock = pinmeter_platform::shared::SystemClock::default();
            let mut collector = None;
            loop {
                let generation = match s.sampling.wait() {
                    SamplingAction::Stop => break,
                    SamplingAction::Reset => {
                        collector = None;
                        s.data.lock().unwrap().monitor.reset();
                        continue;
                    }
                    SamplingAction::Collect(generation) => generation,
                };
                let result = if collector.is_none() {
                    match pinmeter_platform::disk::DiskCollector::new() {
                        Ok(c) => {
                            collector = Some(c);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Ok(())
                }
                .and_then(|_| collector.as_mut().unwrap().sample());
                if result.as_ref().is_err_and(|e| e.status != Status::Warming) {
                    collector = None;
                }
                s.sampling.finish(generation, || {
                    s.data.lock().unwrap().monitor.accept(
                        result,
                        clock.monotonic_ms(),
                        clock.wall_ms(),
                    );
                });
            }
        }));
        service
    }
    pub fn stop(&self) {
        self.sampling.stop();
        if let Some(w) = self.worker.lock().unwrap().take() {
            let _ = w.join();
        }
    }
    pub fn set_visible(&self, visible: bool) {
        self.sampling.set_visible(visible);
    }
    fn snapshot(&self, id: Option<String>, visible: bool) -> DiskSnapshotDto {
        if visible {
            self.sampling.request();
        }
        let data = self.data.lock().unwrap();
        let m = &data.monitor;
        let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
        let stale = m
            .frames
            .back()
            .is_some_and(|f| now.saturating_sub(f.mono) > 6_000);
        let map = |d: &pinmeter_core::disk::DiskPoint| DiskPointDto {
            id: d.id.clone(),
            read: presenters::reading(&d.read, true),
            write: presenters::reading(&d.write, true),
            activity: presenters::reading(&d.activity, false),
        };
        let selected_id = id.or_else(|| {
            m.frames
                .back()
                .and_then(|f| f.disks.first())
                .map(|d| d.id.clone())
        });
        let devices = m
            .frames
            .back()
            .map(|f| {
                f.disks
                    .iter()
                    .map(|d| {
                        let mut dto = map(d);
                        if stale {
                            for r in [&mut dto.read, &mut dto.write, &mut dto.activity] {
                                r.status = ReadingStatus::Stale;
                                r.value = None;
                                r.text = "—".into();
                            }
                        }
                        dto
                    })
                    .collect()
            })
            .unwrap_or_default();
        DiskSnapshotDto {
            status: presenters::status(if stale { Status::Warming } else { m.status }),
            detail: if stale {
                "采样已暂停，重新建立磁盘基线".into()
            } else {
                m.detail.clone()
            },
            devices,
            history: m
                .frames
                .iter()
                .filter(|f| now.saturating_sub(f.mono) <= 300_000)
                .map(|f| DiskFrameDto {
                    at_ms: f.wall as f64,
                    elapsed_ms: f.mono as f64,
                    generation: f.generation.to_string(),
                    point: f
                        .disks
                        .iter()
                        .find(|d| Some(&d.id) == selected_id.as_ref())
                        .map(map),
                })
                .collect(),
            selected_id,
        }
    }
}
#[tauri::command]
pub fn get_disk_snapshot(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, Arc<Runtime>>,
    disk_id: Option<String>,
) -> Result<DiskSnapshotDto, String> {
    if window.label() != "main" {
        return Err("仅主窗口可以查看磁盘".into());
    }
    if disk_id.as_ref().is_some_and(|id| id.len() > 256) {
        return Err("磁盘标识过长".into());
    }
    runtime
        .disks
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.snapshot(disk_id, runtime.visible.load(Ordering::Acquire)))
        .ok_or_else(|| "磁盘服务正在启动".into())
}
