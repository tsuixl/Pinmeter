//! Native desktop display adapter. It consumes projections, never samples metrics.
use pinmeter_core::desktop::{DesktopIntent, DesktopStatus, DesktopSummary};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, sync_channel},
};

#[cfg(target_os = "windows")]
mod render;
#[cfg(target_os = "windows")]
mod windows;

pub struct Taskbar {
    latest: Arc<Mutex<Option<DesktopSummary>>>,
    status: Arc<Mutex<DesktopStatus>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    pub actions: Receiver<DesktopIntent>,
}
impl Default for Taskbar {
    fn default() -> Self {
        Self::start()
    }
}
impl Taskbar {
    pub fn start() -> Self {
        let latest = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new(DesktopStatus {
            supported: cfg!(target_os = "windows"),
            stage: "disabled".into(),
            detail: "任务栏显示已关闭".into(),
            revision: 0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, actions) = sync_channel(16);
        let worker = spawn(latest.clone(), status.clone(), stop.clone(), sender);
        Self {
            latest,
            status,
            stop,
            worker,
            actions,
        }
    }
    pub fn submit(&self, summary: DesktopSummary) {
        *self.latest.lock().unwrap() = Some(summary);
    }
    pub fn finished(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.is_finished())
    }
    pub fn status(&self) -> DesktopStatus {
        self.status.lock().unwrap().clone()
    }
}
impl Drop for Taskbar {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(target_os = "windows")]
fn spawn(
    latest: Arc<Mutex<Option<DesktopSummary>>>,
    status: Arc<Mutex<DesktopStatus>>,
    stop: Arc<AtomicBool>,
    sender: SyncSender<DesktopIntent>,
) -> Option<std::thread::JoinHandle<()>> {
    Some(std::thread::spawn(move || {
        windows::run(latest, status, stop, sender)
    }))
}
#[cfg(not(target_os = "windows"))]
fn spawn(
    _: Arc<Mutex<Option<DesktopSummary>>>,
    status: Arc<Mutex<DesktopStatus>>,
    _: Arc<AtomicBool>,
    _: SyncSender<DesktopIntent>,
) -> Option<std::thread::JoinHandle<()>> {
    *status.lock().unwrap() = DesktopStatus {
        supported: false,
        stage: "unsupported".into(),
        detail: "此平台暂不支持任务栏直显".into(),
        revision: 0,
    };
    None
}
