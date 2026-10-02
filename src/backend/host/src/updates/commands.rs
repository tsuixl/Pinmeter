use super::{Updates, dto::UpdateSnapshotDto};
use crate::runtime::Runtime;
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};
fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("此窗口不可操作应用更新".into())
    }
}
#[tauri::command]
pub fn get_update_state(
    window: WebviewWindow,
    updates: State<'_, Arc<Updates>>,
) -> Result<UpdateSnapshotDto, String> {
    authorize(&window)?;
    Ok(updates.snapshot())
}
#[tauri::command]
pub async fn check_app_update(
    window: WebviewWindow,
    updates: State<'_, Arc<Updates>>,
) -> Result<(), String> {
    authorize(&window)?;
    updates
        .inner()
        .clone()
        .check(window.app_handle(), true)
        .await
}
#[tauri::command]
pub async fn download_app_update(
    window: WebviewWindow,
    updates: State<'_, Arc<Updates>>,
) -> Result<(), String> {
    authorize(&window)?;
    updates.inner().clone().download(window.app_handle()).await
}
#[tauri::command]
pub async fn install_app_update(
    window: WebviewWindow,
    updates: State<'_, Arc<Updates>>,
    runtime: State<'_, Arc<Runtime>>,
    confirmed: bool,
) -> Result<(), String> {
    authorize(&window)?;
    let updates = updates.inner().clone();
    let runtime = runtime.inner().clone();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || updates.install(&app, &runtime, confirmed))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn update_update_preference(
    window: WebviewWindow,
    updates: State<'_, Arc<Updates>>,
    action: String,
    value: Option<bool>,
) -> Result<(), String> {
    authorize(&window)?;
    let updates = updates.inner().clone();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || updates.preference(&app, &action, value))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn open_update_downloads(window: WebviewWindow) -> Result<(), String> {
    authorize(&window)?;
    pinmeter_platform::updates::open_downloads()
}
