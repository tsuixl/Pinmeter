use crate::{contracts::*, presenters, runtime::Runtime};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
#[tauri::command]
pub fn get_hardware_info(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<crate::hardware::HardwareInfoDto, String> {
    authorize(&window)?;
    Ok(crate::hardware::snapshot(
        &*runtime.hardware.lock().map_err(|e| e.to_string())?,
    ))
}
#[tauri::command]
pub fn minimize_to_tray(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    authorize(&window)?;
    runtime.minimize_to_tray(window.app_handle())
}
#[tauri::command]
pub async fn resolve_app_close(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    action: String,
    remember: bool,
) -> Result<(), String> {
    authorize(&window)?;
    let runtime = runtime.inner().clone();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.resolve_close(&app, &action, remember))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn get_app_exit_state(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<crate::exit::ExitStatusDto, String> {
    authorize(&window)?;
    Ok(runtime.exit_status())
}
#[tauri::command]
pub fn request_app_exit(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    confirmed: bool,
) -> Result<(), String> {
    authorize(&window)?;
    runtime.inner().begin_exit(window.app_handle(), confirmed);
    Ok(())
}
#[tauri::command]
pub fn cancel_app_exit(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    authorize(&window)?;
    runtime.cancel_exit(window.app_handle())
}
#[tauri::command]
pub async fn release_all_network_control(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    expected_revision: String,
) -> Result<(), String> {
    authorize(&window)?;
    if runtime.exit_status().stage != "idle" {
        return Err("正在处理退出".into());
    }
    let expected = expected_revision.parse().map_err(|_| "无效的规则版本")?;
    let control = runtime
        .control
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("网络控制尚未准备好")?;
    tauri::async_runtime::spawn_blocking(move || control.release_all(Some(expected), false))
        .await
        .map_err(|e| e.to_string())?
}
pub(crate) fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("窗口无权访问监控数据".into())
    }
}

#[tauri::command]
pub async fn temperature_driver_missing(window: WebviewWindow) -> Result<bool, String> {
    authorize(&window)?;
    let helper = window
        .app_handle()
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("sensors/pinmeter-sensors.exe");
    tauri::async_runtime::spawn_blocking(move || {
        pinmeter_platform::temperature_driver::missing(&helper)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn install_temperature_driver(window: WebviewWindow) -> Result<String, String> {
    authorize(&window)?;
    let helper = window
        .app_handle()
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("sensors/pinmeter-sensors.exe");
    tauri::async_runtime::spawn_blocking(move || {
        pinmeter_platform::temperature_driver::install(&helper)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn change_network_control(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    id: String,
    action: String,
    download: Option<u32>,
    upload: Option<u32>,
    expected_revision: String,
) -> Result<(), String> {
    authorize(&window)?;
    let expected = expected_revision
        .parse::<u64>()
        .map_err(|_| "无效的规则版本")?;
    let control = runtime
        .control
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("网络控制尚未准备好")?;
    if runtime.exit_status().stage != "idle" {
        return Err("正在处理退出，请返回后修改规则".into());
    }
    // Targets must come from backend-observed apps or our existing durable rules.
    let target = control
        .target(&id)
        .or_else(|| {
            runtime
                .inner
                .lock()
                .ok()?
                .monitor
                .app_network
                .apps
                .values()
                .find(|a| a.id == id)
                .map(|a| pinmeter_core::app_network_control::Rule {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    path: a.path.clone(),
                    ..Default::default()
                })
        })
        .ok_or("应用已不在列表中，请重新开始监控或从已配置规则操作")?;
    pinmeter_platform::app_network_control::validate_target(&target)?;
    tauri::async_runtime::spawn_blocking(move || {
        control.change(target, action, download, upload, expected)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_ip_view_active(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    active: bool,
) -> Result<(), String> {
    authorize(&window)?;
    if active
        && (window.is_minimized().map_err(|e| e.to_string())?
            || !window.is_visible().map_err(|e| e.to_string())?)
    {
        return Err("请打开可见的 IP 页面".into());
    }
    runtime.set_ip_active(window.label(), active)
}
#[tauri::command]
pub fn refresh_ip(window: WebviewWindow, runtime: State<'_, Arc<Runtime>>) -> Result<(), String> {
    authorize(&window)?;
    if window.is_minimized().map_err(|e| e.to_string())?
        || !window.is_visible().map_err(|e| e.to_string())?
    {
        return Err("请打开可见的 IP 页面".into());
    }
    let mut state = runtime.inner.lock().map_err(|e| e.to_string())?;
    state.ip.begin(true, crate::ip::now())?;
    state.ip.refresh_checks(None, crate::ip::now())
}

#[tauri::command]
pub fn refresh_ip_checks(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    section: String,
) -> Result<(), String> {
    authorize(&window)?;
    if window.is_minimized().map_err(|e| e.to_string())?
        || !window.is_visible().map_err(|e| e.to_string())?
    {
        return Err("请打开可见的 IP 页面".into());
    }
    let kind = pinmeter_core::ip::checks::CheckKind::ALL
        .into_iter()
        .find(|kind| kind.id() == section)
        .ok_or("未知检测分组")?;
    runtime
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .ip
        .refresh_checks(Some(kind), crate::ip::now())
}

#[tauri::command]
pub fn set_app_network_monitoring(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    enabled: bool,
) -> Result<(), String> {
    authorize(&window)?;
    if enabled
        && (window.is_minimized().map_err(|e| e.to_string())?
            || !window.is_visible().map_err(|e| e.to_string())?)
    {
        return Err("请在可见的网络详情页开始监控".into());
    }
    runtime.set_app_network(enabled)
}

#[tauri::command]
pub fn get_desktop_platform(window: WebviewWindow) -> Result<&'static str, String> {
    authorize(&window)?;
    Ok(std::env::consts::OS)
}

#[tauri::command]
pub fn perform_desktop_action(window: WebviewWindow, action: String) -> Result<(), String> {
    authorize(&window)?;
    match action.as_str() {
        "recover_window" => crate::bridges::ensure_visible(&window, true),
        "minimize" => window.minimize(),
        "activate" => crate::bridges::activate(&window),
        _ => return Err("不支持的窗口操作".into()),
    }
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_settings(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    settings: SettingsDto,
    expected_revision: String,
) -> Result<SettingsDto, String> {
    authorize(&window)?;
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        save_settings(&runtime, settings, expected_revision)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn save_settings(
    runtime: &Runtime,
    settings: SettingsDto,
    expected_revision: String,
) -> Result<SettingsDto, String> {
    let expected = expected_revision
        .parse::<u64>()
        .map_err(|_| "无效的配置版本")?;
    if settings.revision != expected_revision {
        return Err("配置版本不一致".into());
    }
    let next = pinmeter_core::domain::Settings {
        start_in_tray: settings.start_in_tray,
        record_app_traffic_on_start: settings.record_app_traffic_on_start,
        autostart: settings.autostart,
        close_action: settings.close_action,
        taskbar: pinmeter_core::desktop::TaskbarSettings {
            enabled: settings.taskbar.enabled,
            hidden: settings.taskbar.hidden,
            layout: settings.taskbar.layout,
            cpu: settings.taskbar.cpu,
            gpu: settings.taskbar.gpu,
            memory: settings.taskbar.memory,
            network: settings.taskbar.network,
            cpu_temperature: settings.taskbar.cpu_temperature,
            gpu_temperature: settings.taskbar.gpu_temperature,
            gpu_id: settings.taskbar.gpu_id,
            order: settings.taskbar.order,
        },
        schema_version: 1,
        revision: expected,
        theme: settings.theme,
        font_family: settings.font_family,
        font_style: settings.font_style,
        interval_ms: settings.interval_ms as u64,
        network_id: settings.network_id,
        release_network_on_exit: settings.release_network_on_exit,
    };
    crate::fonts::validate_change(runtime, &next)?;
    let saved = runtime.save_settings("idle", |_| Ok((next, expected)))?;
    Ok(presenters::settings(&saved))
}
#[tauri::command]
pub fn get_monitor_state(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<MonitorStateDto, String> {
    authorize(&window)?;
    let state = runtime.inner.lock().map_err(|e| e.to_string())?;
    let mut dto = presenters::state(&state.monitor, &runtime.session, 0);
    dto.ip = Some(crate::ip::dto::snapshot(&state.ip));
    dto.network_control = runtime.control_snapshot();
    Ok(dto)
}
#[tauri::command]
pub fn subscribe_monitor(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    channel: Channel<MonitorBatchDto>,
) -> Result<String, String> {
    authorize(&window)?;
    if window.is_minimized().map_err(|e| e.to_string())? {
        return Err("窗口已最小化".into());
    }
    runtime.subscribe(channel)
}
#[tauri::command]
pub fn unsubscribe_monitor(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    subscription_id: String,
) -> Result<(), String> {
    authorize(&window)?;
    let mut state = runtime.inner.lock().map_err(|e| e.to_string())?;
    if state
        .subscription
        .as_ref()
        .is_some_and(|s| s.id == subscription_id)
    {
        state.subscription = None;
        state.ip.deactivate();
    }
    Ok(())
}
#[tauri::command]
pub fn ack_monitor_batch(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    subscription_id: String,
    delivery_seq: String,
) -> Result<(), String> {
    authorize(&window)?;
    let seq: u64 = delivery_seq.parse().map_err(|_| "无效的批次序号")?;
    let mut state = runtime.inner.lock().map_err(|e| e.to_string())?;
    if let Some(s) = state
        .subscription
        .as_mut()
        .filter(|s| s.id == subscription_id)
    {
        if seq == s.seq {
            s.acked = true;
        } else if seq > s.seq {
            return Err("确认序号超前".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub fn get_history(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    after: String,
) -> Result<HistoryDto, String> {
    authorize(&window)?;
    let cursor: u64 = after.parse().map_err(|_| "无效的历史游标")?;
    let state = runtime.inner.lock().map_err(|e| e.to_string())?;
    let first = state.monitor.history.front().map_or(0, |f| f.cursor);
    let last = state.monitor.history.back().map_or(0, |f| f.cursor);
    Ok(HistoryDto {
        session_id: runtime.session.clone(),
        frames: state
            .monitor
            .history
            .iter()
            .filter(|f| f.cursor > cursor)
            .map(presenters::frame)
            .collect(),
        retained_from: first.to_string(),
        through: last.to_string(),
        missing: cursor > 0 && first > cursor.saturating_add(1),
    })
}
