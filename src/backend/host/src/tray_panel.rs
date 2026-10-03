use crate::{bridges, contracts::FrameDto, fonts::FontCatalogDto, presenters, runtime::Runtime};
use pinmeter_core::ports::Clock;
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use ts_rs::TS;

const LABEL: &str = "tray-panel";
#[derive(Default)]
pub struct PanelState {
    requested: AtomicBool,
    ready: AtomicBool,
}
#[derive(Clone, Serialize, TS)]
pub struct TraySnapshotDto {
    pub frame: Option<FrameDto>,
    pub history: Vec<FrameDto>,
    pub theme: String,
    pub font_family: String,
    pub font_style: String,
    pub network: String,
}
fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == LABEL {
        Ok(())
    } else {
        Err("仅托盘面板可以调用此入口".into())
    }
}
fn hide(window: &WebviewWindow) -> Result<(), String> {
    window
        .app_handle()
        .state::<PanelState>()
        .requested
        .store(false, Ordering::Release);
    let _ = window.emit("tray-panel-visible", false);
    window.hide().map_err(|e| e.to_string())?;
    bridges::set_webview_background(window, true);
    Ok(())
}
pub fn toggle(app: &AppHandle, runtime: &Runtime) -> Result<(), String> {
    let result = toggle_inner(app, runtime);
    if result.is_err() {
        app.state::<PanelState>()
            .requested
            .store(false, Ordering::Release);
    }
    result
}
fn toggle_inner(app: &AppHandle, runtime: &Runtime) -> Result<(), String> {
    let state = app.state::<PanelState>();
    if state.requested.swap(true, Ordering::AcqRel)
        && let Some(window) = app.get_webview_window(LABEL)
    {
        return hide(&window);
    }
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        window
    } else {
        state.ready.store(false, Ordering::Release);
        let window =
            WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html?panel=1".into()))
                .title("Pinmeter 快捷面板")
                .inner_size(420., 360.)
                .resizable(false)
                .decorations(false)
                .visible(false)
                .focused(false)
                .skip_taskbar(true)
                .always_on_top(true)
                .shadow(true)
                .build()
                .map_err(|e| {
                    state.requested.store(false, Ordering::Release);
                    e.to_string()
                })?;
        let panel = window.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::Focused(false) if panel.is_visible().unwrap_or(false) => {
                let _ = hide(&panel);
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = hide(&panel);
            }
            _ => (),
        });
        window
    };
    let theme = runtime
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .monitor
        .settings
        .theme
        .clone();
    bridges::apply_theme(&window, &theme).map_err(|e| e.to_string())?;
    position(&window)?;
    if state.ready.load(Ordering::Acquire) {
        show(&window)?;
    }
    Ok(())
}
fn show(window: &WebviewWindow) -> Result<(), String> {
    if !window
        .app_handle()
        .state::<PanelState>()
        .requested
        .load(Ordering::Acquire)
    {
        return Ok(());
    }
    window.show().map_err(|e| e.to_string())?;
    bridges::set_webview_background(window, false);
    window.set_focus().map_err(|e| e.to_string())?;
    let _ = window.emit("tray-panel-visible", true);
    Ok(())
}
fn position(window: &WebviewWindow) -> Result<(), String> {
    let rect = window
        .app_handle()
        .tray_by_id("pinmeter-resident")
        .and_then(|tray| tray.rect().ok().flatten());
    let anchor = if let Some(rect) = rect {
        rect.position.to_physical::<f64>(1.)
    } else {
        window.cursor_position().map_err(|e| e.to_string())?
    };
    let monitor = window
        .monitor_from_point(anchor.x, anchor.y)
        .map_err(|e| e.to_string())?
        .or(window.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or("没有可用显示器")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (420. * scale).round() as u32;
    let height = (360. * scale).round() as u32;
    let size = PhysicalSize::new(width.min(area.size.width), height.min(area.size.height));
    window.set_size(size).map_err(|e| e.to_string())?;
    let (x, y) = clamped_position(
        (
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        ),
        (anchor.x as i32, anchor.y as i32),
        (size.width, size.height),
    );
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}
fn clamped_position(
    area: (i32, i32, u32, u32),
    anchor: (i32, i32),
    size: (u32, u32),
) -> (i32, i32) {
    let left = area.0 as i64;
    let top = area.1 as i64;
    let right = left + area.2 as i64 - size.0.min(area.2) as i64;
    let bottom = top + area.3 as i64 - size.1.min(area.3) as i64;
    (
        (anchor.0 as i64 - size.0 as i64 + 24).clamp(left, right) as i32,
        (anchor.1 as i64 - size.1 as i64 - 8).clamp(top, bottom) as i32,
    )
}
#[tauri::command]
pub fn get_tray_snapshot(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<TraySnapshotDto, String> {
    authorize(&window)?;
    let state = runtime.inner.lock().map_err(|e| e.to_string())?;
    let monitor = &state.monitor;
    let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
    Ok(TraySnapshotDto {
        frame: monitor.latest_at(now).as_ref().map(presenters::frame),
        history: monitor
            .history
            .iter()
            .filter(|f| now.saturating_sub(f.elapsed_ms) <= 60_000)
            .map(presenters::frame)
            .collect(),
        theme: monitor.settings.theme.clone(),
        font_family: monitor.settings.font_family.clone(),
        font_style: monitor.settings.font_style.clone(),
        network: monitor
            .selected
            .as_ref()
            .map_or_else(|| "暂无可用网卡".into(), |n| n.name.clone()),
    })
}
#[tauri::command]
pub async fn get_tray_font_catalog(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<FontCatalogDto, String> {
    authorize(&window)?;
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let selected = runtime
            .inner
            .lock()
            .map_err(|e| e.to_string())?
            .monitor
            .settings
            .font_family
            .clone();
        let mut catalog = crate::fonts::catalog(&runtime, false)?;
        catalog.families.retain(|f| {
            f.id == selected || matches!(f.id.as_str(), "harmonyos_sans_sc" | "geist" | "system")
        });
        Ok(catalog.into())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn complete_tray_panel(window: WebviewWindow) -> Result<(), String> {
    authorize(&window)?;
    window
        .app_handle()
        .state::<PanelState>()
        .ready
        .store(true, Ordering::Release);
    show(&window)
}
#[tauri::command]
pub fn hide_tray_panel(window: WebviewWindow) -> Result<(), String> {
    authorize(&window)?;
    hide(&window)
}
fn detail_page(page: &str) -> Option<&'static str> {
    match page {
        "overview" => Some("overview"),
        "cpu" => Some("cpu"),
        "memory" => Some("memory"),
        "network" => Some("network"),
        _ => None,
    }
}
#[tauri::command]
pub fn open_tray_detail(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    page: String,
) -> Result<(), String> {
    authorize(&window)?;
    let page = detail_page(&page).ok_or("此面板不支持该操作")?;
    runtime.startup_visibility.lock().unwrap().reveal();
    let result = (|| {
        let main = window
            .app_handle()
            .get_webview_window("main")
            .ok_or("主窗口暂不可用")?;
        bridges::activate(&main).map_err(|e| e.to_string())?;
        main.emit("desktop-navigate", page)
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        // Main activation may already have blurred and hidden this panel.
        window
            .app_handle()
            .state::<PanelState>()
            .requested
            .store(true, Ordering::Release);
        if let Err(recovery) = show(&window) {
            return Err(format!("{error}；快捷面板恢复失败：{recovery}"));
        }
        return Err(error);
    }
    hide(&window)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn position_stays_inside_negative_and_small_work_areas() {
        assert_eq!(
            clamped_position((-1920, 0, 1920, 1040), (-5, 1060), (420, 360)),
            (-420, 680)
        );
        assert_eq!(
            clamped_position((0, 0, 320, 240), (100, 200), (420, 360)),
            (0, 0)
        );
    }
    #[test]
    fn panel_detail_does_not_accept_write_or_control_routes() {
        for page in ["settings", "updates", "exit", "network-control", ""] {
            assert!(detail_page(page).is_none());
        }
        assert_eq!(detail_page("network"), Some("network"));
    }
}
