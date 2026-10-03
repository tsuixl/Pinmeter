use crate::{bridges, runtime::Runtime};
use pinmeter_core::{desktop::*, domain::Status, ports::Clock};
use std::sync::{
    Arc,
    atomic::Ordering,
    mpsc::{Receiver, SyncSender, sync_channel},
};
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};

pub struct Desktop {
    native: Option<pinmeter_platform::taskbar::Taskbar>,
    tray: Option<TrayIcon>,
    tx: SyncSender<DesktopIntent>,
    rx: Receiver<DesktopIntent>,
    attempted_revision: Option<u64>,
    tray_error: Option<String>,
    native_revision: Option<u64>,
    next_tray_check: Instant,
    menu_registered: bool,
    gpu_id: Option<String>,
}
impl Desktop {
    pub fn new() -> Self {
        let (tx, rx) = sync_channel(16);
        Self {
            native: None,
            tray: None,
            tx,
            rx,
            attempted_revision: None,
            tray_error: None,
            native_revision: None,
            next_tray_check: Instant::now(),
            menu_registered: false,
            gpu_id: None,
        }
    }
    pub fn tick(&mut self, runtime: &Arc<Runtime>, app: &AppHandle) {
        if !self.menu_registered {
            self.menu_registered = true;
            if runtime.bind_desktop_actions(self.tx.clone()) {
                let owner = Arc::downgrade(runtime);
                app.on_menu_event(move |_, event| {
                    let intent = match event.id.as_ref() {
                        "cpu" => DesktopIntent::Open("cpu"),
                        "gpu" => DesktopIntent::Open("gpu"),
                        "memory" => DesktopIntent::Open("memory"),
                        "network" => DesktopIntent::Open("network"),
                        "settings" => DesktopIntent::Open("settings"),
                        "updates" => DesktopIntent::Open("updates"),
                        "toggle" => DesktopIntent::ToggleReadings,
                        "exit" => DesktopIntent::Exit,
                        "overview" => DesktopIntent::Open("overview"),
                        _ => return,
                    };
                    if let Some(runtime) = owner.upgrade() {
                        runtime.send_desktop_action(intent);
                    }
                });
            }
        }
        while let Ok(intent) = self.rx.try_recv() {
            act(runtime, app, intent);
        }
        if let Some(native) = &self.native {
            while let Ok(intent) = native.actions.try_recv() {
                act(runtime, app, intent);
            }
        }
        let (revision, enabled) = {
            let state = runtime.inner.lock().unwrap();
            (
                state.monitor.settings.revision,
                state.monitor.settings.taskbar.enabled,
            )
        };
        if Instant::now() >= self.next_tray_check {
            self.next_tray_check = Instant::now() + Duration::from_secs(5);
            if self.tray.as_ref().is_some_and(|tray| {
                cfg!(target_os = "windows") && !matches!(tray.rect(), Ok(Some(_)))
            }) {
                runtime.tray_ready.store(false, Ordering::Release);
                app.remove_tray_by_id("pinmeter-resident");
                self.tray = None;
                self.attempted_revision = None;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }
            // A failed registration is retried slowly; the main window stays available.
            if self.tray.is_none() {
                self.attempted_revision = None;
            }
        }
        if self.native.as_ref().is_some_and(|native| native.finished())
            && self.native_revision != Some(revision)
        {
            self.native = None;
        }
        if self.tray.is_none() && self.attempted_revision != Some(revision) {
            self.attempted_revision = Some(revision);
            match build_tray(app, self.tx.clone()) {
                Ok(tray) => {
                    self.tray = Some(tray);
                    self.tray_error = None;
                    let ready = !cfg!(target_os = "windows")
                        || matches!(self.tray.as_ref().unwrap().rect(), Ok(Some(_)));
                    runtime.tray_ready.store(ready, Ordering::Release);
                    if !ready {
                        self.tray_error = Some("未能确认系统托盘入口".into());
                    }
                }
                Err(error) => {
                    self.tray_error = Some(error.to_string());
                }
            }
        }
        if !enabled || !cfg!(target_os = "windows") {
            self.native = None;
            self.native_revision = None;
            runtime.inner.lock().unwrap().monitor.desktop = DesktopStatus {
                supported: cfg!(target_os = "windows"),
                revision,
                stage: "disabled".into(),
                detail: if cfg!(target_os = "windows") {
                    "任务栏显示已关闭"
                } else {
                    "此平台暂不支持任务栏直显"
                }
                .into(),
            };
            return;
        }
        let summary = summary_at(
            runtime,
            pinmeter_platform::shared::SystemClock::default().monotonic_ms(),
            self.gpu_id.as_deref(),
        );
        self.gpu_id.clone_from(&summary.gpu_id);
        let revision = summary.revision;
        if self.native.is_none() {
            self.native_revision = Some(summary.revision);
            self.native = Some(pinmeter_platform::taskbar::Taskbar::start());
        }
        let native = self.native.as_ref().unwrap();
        native.submit(summary);
        let mut status = native.status();
        if status.revision != revision && !matches!(status.stage.as_str(), "failed" | "unsupported")
        {
            status = DesktopStatus {
                supported: cfg!(target_os = "windows"),
                stage: "connecting".into(),
                detail: "正在应用任务栏设置".into(),
                revision,
            };
        }
        if let Some(error) = &self.tray_error {
            status.detail = format!(
                "{}；托盘入口不可用，暂时无法收起窗口：{error}",
                status.detail
            );
        }
        runtime.inner.lock().unwrap().monitor.desktop = status;
    }
}
fn build_tray(app: &AppHandle, tx: SyncSender<DesktopIntent>) -> tauri::Result<TrayIcon> {
    let overview = MenuItem::with_id(app, "overview", "打开总览", true, None::<&str>)?;
    let network = MenuItem::with_id(app, "network", "网络详情", true, None::<&str>)?;
    let cpu = MenuItem::with_id(app, "cpu", "CPU 详情", true, None::<&str>)?;
    let gpu = MenuItem::with_id(app, "gpu", "GPU 详情", true, None::<&str>)?;
    let memory = MenuItem::with_id(app, "memory", "内存详情", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "检查更新", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏任务栏读数", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出 Pinmeter", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &overview, &network, &cpu, &gpu, &memory, &settings, &updates, &toggle, &exit,
        ],
    )?;
    let click_tx = tx.clone();
    let mut builder = TrayIconBuilder::with_id("pinmeter-resident")
        .tooltip("Pinmeter · 后台监控中")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_, event| {
            if matches!(
                event,
                tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = click_tx.try_send(DesktopIntent::Open("overview"));
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)
}
fn act(runtime: &Arc<Runtime>, app: &AppHandle, intent: DesktopIntent) {
    match intent {
        DesktopIntent::Open(page) => {
            runtime.startup_visibility.lock().unwrap().reveal();
            if page == "updates" {
                let updates = app.state::<Arc<crate::updates::Updates>>().inner().clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = updates.check(&app, true).await;
                });
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = bridges::activate(&window);
                let _ = window.emit(
                    "desktop-navigate",
                    if page == "updates" { "settings" } else { page },
                );
            }
        }
        DesktopIntent::Exit => runtime.begin_exit(app, false),
        DesktopIntent::ToggleReadings => {
            let runtime = runtime.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                // Repeated native clicks never queue unbounded writes or block sampling.
                let Ok(_writer) = runtime.settings_operation.try_lock() else {
                    return;
                };
                if let Err(error) = runtime.save_settings_serialized("idle", |current| {
                    let mut next = current.clone();
                    next.taskbar.hidden = next.taskbar.enabled && !next.taskbar.hidden;
                    next.taskbar.enabled = true;
                    Ok((next, current.revision))
                }) {
                    runtime.inner.lock().unwrap().monitor.diagnostic = Some(error);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = bridges::activate(&window);
                        let _ = window.emit("desktop-navigate", "settings");
                    }
                }
            });
        }
    }
}
fn summary_at(runtime: &Runtime, now: u64, preferred_gpu: Option<&str>) -> DesktopSummary {
    use pinmeter_core::gpu::GpuMetric;
    let state = runtime.inner.lock().unwrap();
    let monitor = &state.monitor;
    let frame = monitor.latest_at(now);
    let gpu = monitor.gpu.at(now);
    // Keep the selected identity through transient failures and changing utilization.
    let device = gpu
        .devices
        .iter()
        .find(|d| {
            Some(d.id.as_str()) == monitor.settings.taskbar.gpu_id.as_deref().or(preferred_gpu)
        })
        .or_else(|| {
            if monitor.settings.taskbar.gpu_id.is_some() {
                return None;
            }
            gpu.devices.iter().reduce(|best, next| {
                let capacity = |device: &pinmeter_core::gpu::GpuDevice| {
                    device
                        .readings
                        .get(&GpuMetric::MemoryTotal)
                        .and_then(|r| r.value)
                        .unwrap_or(0.)
                };
                if capacity(next) > capacity(best) {
                    next
                } else {
                    best
                }
            })
        });
    let mut readings: Vec<_> = [
        ("network", "下载", frame.as_ref().map(|f| &f.download), true),
        ("network", "上传", frame.as_ref().map(|f| &f.upload), true),
        ("cpu", "CPU", frame.as_ref().map(|f| &f.cpu), false),
        ("memory", "内存", frame.as_ref().map(|f| &f.memory), false),
        (
            "gpu",
            "GPU",
            device.and_then(|d| d.readings.get(&GpuMetric::Usage)),
            false,
        ),
    ]
    .into_iter()
    .map(|(key, label, r, network)| project_reading(key, label, r, network, now))
    .collect();
    readings[2].temperature = Some(project_temperature(
        &monitor.cpu_temperature_at(now),
        "CPU 温度",
        false,
        now,
    ));
    if let Some(device) = device {
        readings[4].detail = format!("{} · {}", device.name, readings[4].detail);
        let core = device.readings.get(&GpuMetric::Temperature);
        let soc = device.readings.get(&GpuMetric::VrSocTemperature);
        let alternative = core.is_none_or(|r| r.status == Status::Unsupported)
            && soc.is_some_and(|r| r.status != Status::Unsupported);
        let source = if alternative { soc } else { core };
        let missing = missing_temperature(Status::Unsupported, "此显卡未提供温度");
        readings[4].temperature = Some(project_temperature(
            source.unwrap_or(&missing),
            if alternative {
                "* GPU VR SoC 温度（非核心测温点）"
            } else {
                "GPU 核心温度"
            },
            alternative,
            now,
        ));
    } else {
        let detail = if monitor.settings.taskbar.gpu_id.is_some() {
            "指定显卡当前不可用"
        } else {
            &gpu.detail
        };
        readings[4].detail = detail.into();
        let missing = missing_temperature(gpu.status, detail);
        readings[4].temperature = Some(project_temperature(&missing, "GPU 温度", false, now));
    }
    for (index, show, temperature) in [
        (
            2,
            monitor.settings.taskbar.cpu,
            monitor.settings.taskbar.cpu_temperature,
        ),
        (
            4,
            monitor.settings.taskbar.gpu,
            monitor.settings.taskbar.gpu_temperature,
        ),
    ] {
        readings[index].show_value = show;
        if !temperature {
            readings[index].temperature = None;
        }
        if !show {
            readings[index].valid_at_ms = readings[index]
                .temperature
                .as_ref()
                .and_then(|t| t.valid_at_ms);
            readings[index].normal = readings[index]
                .temperature
                .as_ref()
                .is_some_and(|t| t.normal);
            readings[index].detail = "仅显示温度".into();
        }
    }
    DesktopSummary {
        session: runtime.session.clone(),
        cursor: frame.as_ref().map_or(0, |f| f.cursor),
        network_id: frame.as_ref().and_then(|f| f.network_id.clone()),
        gpu_id: monitor
            .settings
            .taskbar
            .gpu_id
            .clone()
            .or_else(|| device.map(|d| d.id.clone())),
        revision: monitor.settings.revision,
        settings: monitor.settings.taskbar.clone(),
        network: monitor
            .selected
            .as_ref()
            .map_or("暂无可用网卡".into(), |n| {
                format!("网络接口：{}", n.name)
            }),
        readings,
    }
}
fn project_reading(
    key: &'static str,
    label: &'static str,
    reading: Option<&pinmeter_core::domain::Reading>,
    network: bool,
    now: u64,
) -> SummaryReading {
    let Some(r) = reading else {
        return SummaryReading {
            show_value: true,
            key,
            label,
            valid_at_ms: None,
            text: "—".into(),
            unit: if network { "B/s" } else { "%" }.into(),
            detail: "正在采样".into(),
            normal: false,
            temperature: None,
        };
    };
    let dto = crate::presenters::reading(r, network);
    let normal = r.status == Status::Normal && r.value.is_some_and(f64::is_finite);
    let text = if normal && !network {
        format!("{:.0}", r.value.unwrap())
    } else if normal {
        dto.text
    } else {
        "—".into()
    };
    let status = match r.status {
        Status::Normal => "实时",
        Status::Warming => "正在采样",
        Status::Unsupported => "不支持",
        Status::PermissionDenied => "权限不足",
        Status::Failed => "采集失败",
        Status::Stale => "数据过期",
    };
    let mut detail = if r.detail.is_empty() {
        status.into()
    } else {
        format!("{status} · {}", r.detail)
    };
    if let Some(at) = r.valid_mono_ms {
        detail = format!(
            "{detail} · 上次有效采样 {} 秒前",
            now.saturating_sub(at) / 1000
        );
    }
    SummaryReading {
        show_value: true,
        valid_at_ms: r.valid_at_ms,
        key,
        label,
        text,
        unit: dto.unit,
        detail,
        normal,
        temperature: None,
    }
}
fn missing_temperature(status: Status, detail: &str) -> pinmeter_core::domain::Reading {
    pinmeter_core::domain::Reading {
        value: None,
        status,
        detail: detail.into(),
        valid_at_ms: None,
        valid_mono_ms: None,
        source: "GPU",
        semantic: "gpu.core_celsius",
    }
}
fn project_temperature(
    r: &pinmeter_core::domain::Reading,
    label: &str,
    alternative_sensor: bool,
    now: u64,
) -> SummaryTemperature {
    let value = project_reading("temperature", "温度", Some(r), false, now);
    SummaryTemperature {
        text: value.text,
        normal: value.normal,
        detail: format!("{label} · {}", value.detail),
        valid_at_ms: value.valid_at_ms,
        alternative_sensor,
    }
}

pub fn ensure_tray(app: &AppHandle) -> Result<(), String> {
    let tray = app
        .tray_by_id("pinmeter-resident")
        .ok_or("托盘尚未就绪，请稍后重试；窗口仍保持打开")?;
    if cfg!(target_os = "windows") && !matches!(tray.rect(), Ok(Some(_))) {
        return Err("托盘入口不可用，未隐藏窗口，请稍后重试".into());
    }
    Ok(())
}
pub fn minimize_to_tray(app: &AppHandle) -> Result<(), String> {
    ensure_tray(app)?;
    app.get_webview_window("main")
        .ok_or("主窗口不可用")?
        .hide()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn temperature_only_and_missing_selected_gpu_do_not_show_other_readings() {
        let runtime = Runtime::new(Arc::new(Repository));
        {
            let mut state = runtime.inner.lock().unwrap();
            state.monitor.settings.taskbar.cpu = false;
            state.monitor.settings.taskbar.gpu_id = Some("missing-device".into());
            state.monitor.accept_temperature(Observation {
                result: Ok(65.),
                mono_ms: 1000,
                wall_ms: 1000,
                source: "test",
                semantic: "test",
            });
        }
        let summary = summary_at(&runtime, 1000, None);
        assert!(!summary.readings[2].show_value);
        assert!(summary.readings[2].display().contains("65°C"));
        assert!(!summary.readings[2].display().contains('%'));
        assert_eq!(summary.gpu_id.as_deref(), Some("missing-device"));
        assert!(!summary.readings[4].normal);
        assert!(summary.readings[4].detail.contains("指定显卡"));
    }
    #[test]
    fn restarting_desktop_routes_to_the_new_bounded_queue_without_registering_again() {
        let runtime = Runtime::new(Arc::new(Repository));
        let (old_tx, old_rx) = sync_channel(1);
        assert!(runtime.bind_desktop_actions(old_tx));
        runtime.send_desktop_action(DesktopIntent::Open("cpu"));
        assert!(matches!(old_rx.try_recv(), Ok(DesktopIntent::Open("cpu"))));
        let (new_tx, new_rx) = sync_channel(1);
        assert!(!runtime.bind_desktop_actions(new_tx));
        runtime.send_desktop_action(DesktopIntent::Open("memory"));
        runtime.send_desktop_action(DesktopIntent::Exit);
        assert!(matches!(
            old_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        ));
        assert!(matches!(
            new_rx.try_recv(),
            Ok(DesktopIntent::Open("memory"))
        ));
        assert!(matches!(
            new_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        runtime.stop();
        runtime.send_desktop_action(DesktopIntent::Open("cpu"));
        assert!(matches!(
            new_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        ));
    }
    use super::*;
    use pinmeter_core::{domain::*, ports::SettingsRepository};
    struct Repository;
    impl SettingsRepository for Repository {
        fn load(&self) -> Result<Option<Settings>, String> {
            Ok(None)
        }
        fn save(&self, _: &Settings) -> Result<(), String> {
            Ok(())
        }
    }
    #[test]
    fn desktop_uses_the_same_frame_and_never_turns_missing_or_stale_data_into_zero() {
        let runtime = Runtime::new(Arc::new(Repository));
        assert!(
            summary_at(
                &runtime,
                pinmeter_platform::shared::SystemClock::default().monotonic_ms(),
                None
            )
            .readings
            .iter()
            .all(|r| !r.normal && r.text == "—")
        );
        let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
        let valid = Reading {
            value: Some(0.),
            status: Status::Normal,
            valid_at_ms: Some(123),
            valid_mono_ms: Some(now),
            source: "fixture",
            semantic: "test",
            detail: String::new(),
        };
        let mut cpu = valid.clone();
        cpu.value = Some(12.4);
        let mut memory = valid.clone();
        memory.status = Status::Failed;
        memory.value = Some(46.);
        runtime
            .inner
            .lock()
            .unwrap()
            .monitor
            .history
            .push_back(Frame {
                cursor: 42,
                at_ms: 123,
                elapsed_ms: now,
                generation: 1,
                network_generation: 1,
                network_id: Some("adapter".into()),
                cpu,
                cpu_temperature: valid.clone(),
                memory,
                download: valid.clone(),
                upload: valid,
                gpus: vec![],
                memory_used: None,
                memory_total: None,
            });
        let projected = summary_at(
            &runtime,
            pinmeter_platform::shared::SystemClock::default().monotonic_ms(),
            None,
        );
        assert_eq!(projected.session, runtime.session);
        assert_eq!(projected.cursor, 42);
        assert_eq!(projected.network_id.as_deref(), Some("adapter"));
        assert_eq!(projected.readings[0].text, "0.0");
        assert!(projected.readings[0].normal);
        assert_eq!(projected.readings[0].valid_at_ms, Some(123));
        assert_eq!(projected.readings[2].text, "12");
        assert_eq!(projected.readings[3].text, "—");
        assert!(projected.readings[3].detail.contains("采集失败"));
        {
            let mut state = runtime.inner.lock().unwrap();
            let frame = state.monitor.history.back_mut().unwrap();
            frame.download.valid_mono_ms = Some(now.saturating_sub(60_000));
        }
        let stale = summary_at(&runtime, now + 60_000, None);
        assert_eq!(stale.readings[0].text, "—");
        assert!(stale.readings[0].detail.contains("数据过期"));
        assert_eq!(stale.readings[0].valid_at_ms, Some(123));
    }
    #[test]
    fn hardware_temperatures_keep_status_sensor_and_gpu_identity() {
        use pinmeter_core::gpu::{GpuMetric, GpuSample};
        let runtime = Runtime::new(Arc::new(Repository));
        let gpu_sample = |at, devices| Observation {
            result: Ok(devices),
            source: "fixture",
            semantic: "gpu",
            mono_ms: at,
            wall_ms: at + 100,
        };
        let device = |id: &str, capacity, usage, temperature| GpuSample {
            id: id.into(),
            name: format!("Device {id}"),
            readings: [
                (GpuMetric::MemoryTotal, Ok(capacity)),
                (GpuMetric::Usage, Ok(usage)),
                (GpuMetric::Temperature, temperature),
                (GpuMetric::VrSocTemperature, Ok(61.)),
            ]
            .into_iter()
            .collect(),
        };
        {
            let mut state = runtime.inner.lock().unwrap();
            state.monitor.accept_temperature(Observation {
                result: Ok(57.4),
                source: "fixture",
                semantic: "cpu.package_celsius",
                mono_ms: 1000,
                wall_ms: 1100,
            });
            state.monitor.gpu.accept(gpu_sample(
                1000,
                vec![
                    device(
                        "small",
                        8e9,
                        80.,
                        Err(Failure::new(Status::Unsupported, "No core sensor")),
                    ),
                    device("large", 24e9, 0., Ok(0.)),
                ],
            ));
        }
        let initial = summary_at(&runtime, 1000, None);
        assert_eq!(initial.gpu_id.as_deref(), Some("large"));
        assert_eq!(
            initial.readings[2]
                .temperature
                .as_ref()
                .unwrap()
                .formatted(),
            "(57°C)"
        );
        let gpu = &initial.readings[4];
        assert_eq!(gpu.display(), "GPU 0% (0°C)");
        assert!(gpu.normal && gpu.temperature.as_ref().unwrap().normal);
        assert!(gpu.detail.contains("Device large"));
        assert_eq!(gpu.temperature.as_ref().unwrap().valid_at_ms, Some(1100));
        runtime.inner.lock().unwrap().monitor.gpu.accept(gpu_sample(
            2000,
            vec![
                device(
                    "small",
                    48e9,
                    99.,
                    Err(Failure::new(Status::Unsupported, "No core sensor")),
                ),
                device(
                    "large",
                    24e9,
                    18.,
                    Err(Failure::new(Status::Failed, "core read failed")),
                ),
            ],
        ));
        let failed = summary_at(&runtime, 2000, initial.gpu_id.as_deref());
        assert_eq!(failed.gpu_id, initial.gpu_id);
        assert_eq!(failed.readings[4].display(), "GPU 18% (—°C)");
        assert!(
            !failed.readings[4]
                .temperature
                .as_ref()
                .unwrap()
                .alternative_sensor
        );
        let stale = summary_at(&runtime, 6000, failed.gpu_id.as_deref());
        assert_eq!(
            stale.readings[2].temperature.as_ref().unwrap().formatted(),
            "(—°C)"
        );
        assert!(
            stale.readings[2]
                .temperature
                .as_ref()
                .unwrap()
                .detail
                .contains("过期")
        );
        assert_eq!(stale.readings[4].display(), "GPU —% (—°C)");
        runtime.inner.lock().unwrap().monitor.gpu.accept(gpu_sample(
            7000,
            vec![device(
                "small",
                48e9,
                2.,
                Err(Failure::new(Status::Unsupported, "No core sensor")),
            )],
        ));
        let removed = summary_at(&runtime, 7000, initial.gpu_id.as_deref());
        assert_eq!(removed.gpu_id.as_deref(), Some("small"));
        assert_eq!(removed.readings[4].display(), "GPU 2% (61°C*)");
        assert!(
            removed.readings[4]
                .temperature
                .as_ref()
                .unwrap()
                .detail
                .contains("VR SoC")
        );
        runtime
            .inner
            .lock()
            .unwrap()
            .monitor
            .gpu
            .accept(gpu_sample(8000, vec![]));
        let missing = summary_at(&runtime, 8000, removed.gpu_id.as_deref());
        assert!(missing.gpu_id.is_none());
        assert!(!missing.readings[4].normal);
        assert_eq!(
            missing.readings[4]
                .temperature
                .as_ref()
                .unwrap()
                .formatted(),
            "(—°C)"
        );
    }
}
