use crate::{bridges, desktop, runtime::Runtime};
use pinmeter_core::desktop::StartupAction;
use std::{
    sync::{Arc, atomic::Ordering, mpsc},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[tauri::command]
pub async fn complete_window_startup(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("此窗口不能完成主窗口启动".into());
    }
    let runtime = runtime.inner().clone();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || finish_startup(runtime, app))
        .await
        .map_err(|e| e.to_string())?
}
fn finish_startup(runtime: Arc<Runtime>, app: AppHandle) -> Result<(), String> {
    let requested = runtime.inner.lock().unwrap().monitor.settings.start_in_tray && cfg!(windows);
    let deadline = Instant::now() + Duration::from_secs(3);
    while requested
        && !runtime.tray_ready.load(Ordering::Acquire)
        && Instant::now() < deadline
        && !runtime.is_stopped()
    {
        std::thread::sleep(Duration::from_millis(25));
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = (|| {
            if runtime.is_stopped() {
                return Ok(());
            }
            let window = handle.get_webview_window("main").ok_or("主窗口不可用")?;
            let ready = runtime.tray_ready.load(Ordering::Acquire);
            let action = runtime
                .startup_visibility
                .lock()
                .unwrap()
                .complete(requested, ready);
            match action {
                StartupAction::KeepCurrent => Ok(()),
                StartupAction::Tray => {
                    if let Err(error) = desktop::minimize_to_tray(&handle) {
                        runtime.inner.lock().unwrap().monitor.diagnostic =
                            Some(format!("无法启动到托盘，已显示主窗口：{error}"));
                        bridges::activate(&window).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                }
                StartupAction::Show => {
                    if requested && !ready {
                        let mut state = runtime.inner.lock().unwrap();
                        let detail = "托盘尚未就绪，已显示主窗口";
                        state.monitor.diagnostic = Some(
                            state
                                .monitor
                                .diagnostic
                                .as_ref()
                                .map_or(detail.into(), |previous| format!("{previous}；{detail}")),
                        );
                    }
                    bridges::activate(&window).map_err(|e| e.to_string())
                }
            }
        })();
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| "主窗口启动响应超时".to_string())?
}
