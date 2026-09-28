use crate::{contracts::*, presenters};
use pinmeter_core::ports::SettingsRepository;
use pinmeter_core::{application::Monitor, domain::Settings};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, ipc::Channel};

pub struct Subscription {
    pub desktop: pinmeter_core::desktop::DesktopStatus,
    pub control_version: String,
    pub ip_revision: u64,
    pub network_revision: u64,
    pub id: String,
    pub channel: Channel<MonitorBatchDto>,
    pub seq: u64,
    pub acked: bool,
    pub sent_at: Instant,
    pub cursor: u64,
    pub revision: u64,
}

pub struct RuntimeState {
    pub ip: pinmeter_core::ip::IpInspection,
    pub monitor: Monitor,
    pub subscription: Option<Subscription>,
    pub next_subscription: u64,
}
pub struct Runtime {
    pub settings_operation: Mutex<()>,
    pub hardware: Mutex<pinmeter_core::hardware::HardwareState>,
    hardware_worker: Mutex<Option<JoinHandle<()>>>,
    pub autostart: pinmeter_platform::autostart::SystemAutostart,
    pub tray_ready: AtomicBool,
    pub exit: Mutex<crate::exit::ExitStatusDto>,
    pub control: Mutex<Option<Arc<crate::network_control::NetworkControl>>>,
    pub inner: Mutex<RuntimeState>,
    pub session: String,
    pub repository: Arc<dyn SettingsRepository>,
    stop: AtomicBool,
    stopped: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Runtime {
    pub fn new(repository: Arc<dyn SettingsRepository>) -> Arc<Self> {
        let (settings, diagnostic) = match repository.load() {
            Ok(Some(settings)) => (settings, None),
            Ok(None) => (Settings::default(), None),
            Err(error) => (
                Settings::default(),
                Some(format!("{error}；本次使用默认设置，原文件已保留")),
            ),
        };
        let mut monitor = Monitor::new(settings, diagnostic);
        use pinmeter_core::autostart::Autostart;
        let autostart = pinmeter_platform::autostart::SystemAutostart;
        monitor.autostart = autostart.status();
        monitor.cpu_model = pinmeter_platform::cpu_model();
        monitor.app_network =
            pinmeter_core::app_network::AppNetwork::new(cfg!(target_os = "windows"));
        Arc::new(Self {
            settings_operation: Mutex::new(()),
            hardware: Mutex::new(pinmeter_core::hardware::HardwareState::Loading),
            hardware_worker: Mutex::new(None),
            autostart,
            tray_ready: AtomicBool::new(false),
            exit: Mutex::new(Default::default()),
            control: Mutex::new(None),
            repository,
            inner: Mutex::new(RuntimeState {
                ip: pinmeter_core::ip::IpInspection::default(),
                monitor,
                subscription: None,
                next_subscription: 0,
            }),
            session: format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            stop: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            worker: Mutex::new(None),
        })
    }
    pub fn start(self: &Arc<Self>, app: AppHandle) {
        let hardware_runtime = Arc::clone(self);
        *self.hardware_worker.lock().unwrap() = Some(thread::spawn(move || {
            let inventory = pinmeter_platform::hardware::collect(&hardware_runtime.stop);
            *hardware_runtime.hardware.lock().unwrap() = inventory;
        }));
        if let (Ok(config), Ok(resources)) =
            (app.path().app_config_dir(), app.path().resource_dir())
        {
            *self.control.lock().unwrap() = Some(crate::network_control::NetworkControl::start(
                config.join("network-rules.json"),
                resources.join("network-control/pinmeter-network-control.exe"),
            ));
        }
        let runtime = Arc::clone(self);
        let worker = thread::spawn(move || {
            let mut desktop = crate::desktop::Desktop::new();
            // Native handles are created, used and dropped on this one thread.
            let mut provider = pinmeter_platform::provider();
            let mut ip_executor = crate::ip::IpExecutor::new();
            let helper = app
                .path()
                .resource_dir()
                .unwrap_or_default()
                .join("sensors/pinmeter-sensors.exe");
            let temperature = pinmeter_platform::temperature::TemperatureCollector::start(helper);
            let gpu = pinmeter_platform::gpu::GpuCollector::start(
                app.path()
                    .resource_dir()
                    .unwrap_or_default()
                    .join("sensors/pinmeter-gpu.exe"),
            );
            let mut next = Instant::now();
            let mut previous = Instant::now();
            let mut previous_wall = SystemTime::now();
            let mut schedule: Option<(u64, Option<String>)> = None;
            let mut visible = true;
            let mut next_display_check = Instant::now();
            let mut display_signature = String::new();
            let mut applied_theme = None;
            let network_helper = app
                .path()
                .resource_dir()
                .unwrap_or_default()
                .join("network/pinmeter-network.exe");
            let mut network: Option<pinmeter_platform::app_network::NetworkCollector> = None;
            while !runtime.stop.load(Ordering::Relaxed) {
                ip_executor.poll(&runtime);
                if pinmeter_platform::startup::launcher_exited() {
                    app.exit(0);
                    break;
                }
                runtime
                    .inner
                    .lock()
                    .unwrap()
                    .monitor
                    .gpu
                    .accept(gpu.latest());
                runtime
                    .inner
                    .lock()
                    .unwrap()
                    .monitor
                    .accept_temperature(temperature.latest());
                if let Some(window) = app.get_webview_window("main") {
                    let theme = runtime.inner.lock().unwrap().monitor.settings.theme.clone();
                    if applied_theme.as_ref() != Some(&theme) {
                        if let Err(error) = crate::bridges::apply_theme(&window, &theme) {
                            eprintln!("Window theme could not be applied: {error}");
                        }
                        applied_theme = Some(theme);
                    }
                    if Instant::now() >= next_display_check {
                        if let Ok(monitors) = window.available_monitors() {
                            let signature = format!("{monitors:?}");
                            if signature != display_signature {
                                display_signature = signature;
                                let _ = crate::bridges::ensure_visible(&window, false);
                            }
                        }
                        next_display_check = Instant::now() + Duration::from_secs(5);
                    }
                    let current = !window.is_minimized().unwrap_or(false)
                        && window.is_visible().unwrap_or(true);
                    if current != visible {
                        visible = current;
                        crate::bridges::set_webview_background(&window, !visible);
                        if !visible {
                            let mut state = runtime.inner.lock().unwrap();
                            state.subscription = None;
                            state.ip.deactivate();
                        }
                        let _ = window.emit("monitor-visibility", visible);
                    }
                }
                let (revision, interval, network_id) = {
                    let state = runtime.inner.lock().unwrap();
                    (
                        state.monitor.settings.revision,
                        state.monitor.settings.interval_ms,
                        state.monitor.settings.network_id.clone(),
                    )
                };
                if let Some(collector) = &network {
                    use pinmeter_core::ports::Clock;
                    collector.interval.store(interval, Ordering::Relaxed);
                    if let Some((generation, result)) = collector.latest() {
                        let mut state = runtime.inner.lock().unwrap();
                        match result {
                            Ok(sample) => state.monitor.app_network.accept(
                                generation,
                                &sample,
                                pinmeter_platform::shared::SystemClock::default().monotonic_ms(),
                                interval,
                            ),
                            Err(error) => state.monitor.app_network.fail(generation, &error),
                        }
                    }
                }
                // Apply failures before deciding to start: only a fresh user request retries.
                if network
                    .as_ref()
                    .is_some_and(|collector| collector.is_finished())
                {
                    // A worker may have failed after the earlier cache read.
                    if let Some((generation, Err(error))) =
                        network.as_ref().and_then(|c| c.latest())
                    {
                        runtime
                            .inner
                            .lock()
                            .unwrap()
                            .monitor
                            .app_network
                            .fail(generation, &error);
                    }
                    network = None;
                }
                let (requested, generation) = {
                    let state = runtime.inner.lock().unwrap();
                    (
                        state.monitor.app_network.requested,
                        state.monitor.app_network.generation,
                    )
                };
                if requested && network.is_none() {
                    network = Some(pinmeter_platform::app_network::NetworkCollector::start(
                        network_helper.clone(),
                        interval,
                        generation,
                    ));
                }
                if let Some(collector) = &network {
                    collector.set_session(if requested { generation } else { 0 });
                }
                if schedule.as_ref() != Some(&(interval, network_id.clone())) {
                    schedule = Some((interval, network_id));
                    provider.reset_baseline();
                    next = Instant::now();
                }
                if Instant::now() >= next {
                    let wall = SystemTime::now();
                    let wall_gap = wall.duration_since(previous_wall).unwrap_or_default();
                    if previous.elapsed() > Duration::from_millis(interval * 3)
                        || wall_gap > Duration::from_millis(interval * 3)
                    {
                        provider.reset_baseline();
                        runtime.inner.lock().unwrap().monitor.reset_baseline();
                        runtime.inner.lock().unwrap().ip.invalidate();
                    }
                    previous = Instant::now();
                    previous_wall = wall;
                    let raw = provider.sample();
                    runtime.inner.lock().unwrap().monitor.accept(raw, revision);
                    next = Instant::now() + Duration::from_millis(interval);
                }
                desktop.tick(&runtime, &app);
                if visible {
                    runtime.send_update();
                }
                thread::park_timeout(Duration::from_millis(100));
            }
        });
        *self.worker.lock().unwrap() = Some(worker);
    }
    pub fn control_snapshot(&self) -> Option<crate::network_control::NetworkControlDto> {
        self.control.lock().unwrap().as_ref().map(|c| c.snapshot())
    }
    pub fn set_app_network(&self, enabled: bool) -> Result<(), String> {
        use pinmeter_core::ports::Clock;
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        if enabled {
            state
                .monitor
                .app_network
                .begin(pinmeter_platform::shared::SystemClock::default().monotonic_ms())?;
        } else {
            state.monitor.app_network.stop();
        }
        Ok(())
    }
    pub fn set_ip_active(&self, window: &str, active: bool) -> Result<(), String> {
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        state.ip.set_active(window, active, crate::ip::now());
        Ok(())
    }
    pub fn request_stop(&self) -> bool {
        if self.stop.swap(true, Ordering::Relaxed) {
            return false;
        }
        self.inner.lock().unwrap().subscription = None;
        if let Some(worker) = self.worker.lock().unwrap().as_ref() {
            worker.thread().unpark();
        }
        true
    }
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
    pub fn stop(&self) {
        if let Some(control) = self.control.lock().unwrap().take() {
            control.stop();
        }
        self.request_stop();
        if let Some(worker) = self.hardware_worker.lock().unwrap().take() {
            let _ = worker.join();
        }
        if let Some(worker) = self.worker.lock().unwrap().take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
        self.stopped.store(true, Ordering::Release);
    }
    pub fn subscribe(&self, channel: Channel<MonitorBatchDto>) -> Result<String, String> {
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        // UI subscription lifetime does not own an explicitly started collection.
        state.ip.deactivate();
        state.next_subscription += 1;
        let id = format!("{}:{}", self.session, state.next_subscription);
        let cursor = state.monitor.history.back().map_or(0, |f| f.cursor);
        let mut dto = presenters::state(&state.monitor, &self.session, 0);
        dto.ip = Some(crate::ip::dto::snapshot(&state.ip));
        dto.network_control = self.control_snapshot();
        channel
            .send(MonitorBatchDto {
                kind: "bootstrap".into(),
                subscription_id: id.clone(),
                delivery_seq: "1".into(),
                history_from: dto.history.first().map_or("0".into(), |f| f.cursor.clone()),
                history_to: cursor.to_string(),
                requires_history_sync: false,
                state: dto,
            })
            .map_err(|e| e.to_string())?;
        state.subscription = Some(Subscription {
            desktop: state.monitor.desktop.clone(),
            control_version: self.control_snapshot().map_or("0".into(), |s| s.version),
            ip_revision: state.ip.revision,
            network_revision: state.monitor.app_network.revision,
            id: id.clone(),
            channel,
            seq: 1,
            acked: false,
            sent_at: Instant::now(),
            cursor,
            revision: state.monitor.settings.revision,
        });
        Ok(id)
    }
    fn send_update(&self) {
        let mut state = self.inner.lock().unwrap();
        let Some(mut subscription) = state.subscription.take() else {
            return;
        };
        if !subscription.acked && subscription.sent_at.elapsed() > Duration::from_secs(15) {
            state.ip.deactivate();
            return;
        }
        let cursor = state.monitor.history.back().map_or(0, |f| f.cursor);
        let revision = state.monitor.settings.revision;
        let network_revision = state.monitor.app_network.revision;
        let ip_revision = state.ip.revision;
        let control = self.control_snapshot();
        let control_version = control.as_ref().map_or("0".into(), |s| s.version.clone());
        if subscription.acked
            && (cursor != subscription.cursor
                || revision != subscription.revision
                || network_revision != subscription.network_revision
                || ip_revision != subscription.ip_revision
                || control_version != subscription.control_version
                || state.monitor.desktop != subscription.desktop)
        {
            let mut dto = presenters::state(&state.monitor, &self.session, subscription.cursor);
            if ip_revision != subscription.ip_revision {
                dto.ip = Some(crate::ip::dto::snapshot(&state.ip));
            }
            dto.network_control = control;
            subscription.seq += 1;
            let missing = state
                .monitor
                .history
                .front()
                .is_some_and(|f| subscription.cursor > 0 && f.cursor > subscription.cursor + 1);
            let batch = MonitorBatchDto {
                kind: "update".into(),
                subscription_id: subscription.id.clone(),
                delivery_seq: subscription.seq.to_string(),
                history_from: subscription.cursor.to_string(),
                history_to: cursor.to_string(),
                requires_history_sync: missing,
                state: dto,
            };
            if subscription.channel.send(batch).is_err() {
                state.ip.deactivate();
                return;
            }
            subscription.acked = false;
            subscription.sent_at = Instant::now();
            subscription.cursor = cursor;
            subscription.revision = revision;
            subscription.network_revision = network_revision;
            subscription.ip_revision = ip_revision;
            subscription.control_version = control_version;
            subscription.desktop = state.monitor.desktop.clone();
        }
        state.subscription = Some(subscription);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ip_projection_uses_existing_backpressure_and_stops_when_view_disconnects() {
        struct Repository;
        impl SettingsRepository for Repository {
            fn load(&self) -> Result<Option<Settings>, String> {
                Ok(None)
            }
            fn save(&self, _: &Settings) -> Result<(), String> {
                Ok(())
            }
        }
        let runtime = Runtime::new(Arc::new(Repository));
        let batches = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let captured = batches.clone();
        runtime
            .subscribe(Channel::new(move |body| {
                if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                    captured
                        .lock()
                        .unwrap()
                        .push(serde_json::from_str(&json).unwrap());
                }
                Ok(())
            }))
            .unwrap();
        assert!(batches.lock().unwrap()[0]["state"].get("ip").is_some());
        {
            let mut state = runtime.inner.lock().unwrap();
            state.subscription.as_mut().unwrap().acked = true;
            state.monitor.settings.revision += 1;
        }
        runtime.send_update();
        assert!(batches.lock().unwrap()[1]["state"].get("ip").is_none());
        runtime.set_ip_active("main", true).unwrap();
        runtime.send_update();
        assert_eq!(batches.lock().unwrap().len(), 2); // Wait for the previous ACK.
        runtime
            .inner
            .lock()
            .unwrap()
            .subscription
            .as_mut()
            .unwrap()
            .acked = true;
        runtime.send_update();
        assert_eq!(batches.lock().unwrap()[2]["state"]["ip"]["running"], true);
        {
            let mut state = runtime.inner.lock().unwrap();
            state.subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(16);
        }
        runtime.send_update();
        let state = runtime.inner.lock().unwrap();
        assert!(state.subscription.is_none());
        assert!(!state.ip.active());
        assert!(!state.ip.running);
    }
    #[test]
    fn subscription_requires_ack_and_replaces_old_channels() {
        struct Repository;
        impl SettingsRepository for Repository {
            fn load(&self) -> Result<Option<Settings>, String> {
                Ok(None)
            }
            fn save(&self, _: &Settings) -> Result<(), String> {
                Ok(())
            }
        }
        let runtime = Runtime::new(Arc::new(Repository));
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = count.clone();
        let id = runtime
            .subscribe(Channel::new(move |_| {
                observed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }))
            .unwrap();
        runtime.inner.lock().unwrap().monitor.settings.revision += 1;
        runtime.send_update();
        assert_eq!(count.load(Ordering::Relaxed), 1);
        runtime
            .inner
            .lock()
            .unwrap()
            .subscription
            .as_mut()
            .unwrap()
            .acked = true;
        runtime.send_update();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        // Native visibility can change without a new sample or settings revision.
        {
            let mut state = runtime.inner.lock().unwrap();
            state.monitor.desktop.stage = "no_space".into();
        }
        runtime.send_update();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        runtime
            .inner
            .lock()
            .unwrap()
            .subscription
            .as_mut()
            .unwrap()
            .acked = true;
        runtime.send_update();
        assert_eq!(count.load(Ordering::Relaxed), 3);
        {
            let mut state = runtime.inner.lock().unwrap();
            state.monitor.app_network = pinmeter_core::app_network::AppNetwork::new(true);
            state.monitor.app_network.begin(0).unwrap();
        }
        assert_ne!(runtime.subscribe(Channel::new(|_| Ok(()))).unwrap(), id);
        let generation = runtime.inner.lock().unwrap().monitor.app_network.generation;
        assert!(runtime.inner.lock().unwrap().monitor.app_network.requested);
        runtime
            .inner
            .lock()
            .unwrap()
            .subscription
            .as_mut()
            .unwrap()
            .sent_at = Instant::now() - Duration::from_secs(16);
        runtime.send_update();
        assert!(runtime.inner.lock().unwrap().subscription.is_none());
        assert!(runtime.inner.lock().unwrap().monitor.app_network.requested);
        assert_eq!(
            runtime.inner.lock().unwrap().monitor.app_network.generation,
            generation
        );
        runtime.set_app_network(false).unwrap();
        assert!(!runtime.inner.lock().unwrap().monitor.app_network.requested);
    }
}
