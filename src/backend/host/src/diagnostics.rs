use crate::{runtime::Runtime, updates::Updates};
use pinmeter_core::{diagnostics::capture, ports::Clock};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, WebviewWindow};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct DiagnosticPreviewDto {
    pub token: String,
    pub generated_at_ms: f64,
    pub content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct DiagnosticExportDto {
    pub saved: bool,
    pub path: Option<String>,
}
#[derive(Default)]
pub struct Diagnostics {
    preview: Mutex<(u64, Option<DiagnosticPreviewDto>)>,
}
fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("此窗口不能访问诊断".into())
    }
}
#[tauri::command]
pub fn prepare_diagnostics(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    updates: State<'_, Arc<Updates>>,
    diagnostics: State<'_, Diagnostics>,
) -> Result<DiagnosticPreviewDto, String> {
    authorize(&window)?;
    let wall = crate::updates::now_ms();
    let mono = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
    let system = pinmeter_platform::diagnostics::system();
    let report = capture(
        &runtime.inner.lock().map_err(|e| e.to_string())?.monitor,
        system,
        env!("CARGO_PKG_VERSION"),
        wall,
        mono,
    );
    let control = runtime.control_snapshot();
    let exit = runtime.exit_status();
    let update = updates.snapshot();
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "report":report,
        "runtime":{"exit_stage":exit.stage,"update_stage":update.stage,
            "network_control_available":control.as_ref().is_some_and(|c|c.available),
            "network_rule_count":control.as_ref().map_or(0,|c|c.rules.len()),
            "has_network_control_diagnostic":control.as_ref().is_some_and(|c|!c.detail.is_empty())},
        "privacy":"不包含路径、用户名、IP、进程名单、规则目标或完整日志"
    }))
    .map_err(|e| e.to_string())?
        + "\n";
    if content.len() > 65536 {
        return Err("诊断内容超出限制".into());
    }
    let mut state = diagnostics.preview.lock().map_err(|e| e.to_string())?;
    state.0 += 1;
    let preview = DiagnosticPreviewDto {
        token: state.0.to_string(),
        generated_at_ms: wall as f64,
        content,
    };
    state.1 = Some(preview.clone());
    Ok(preview)
}
#[tauri::command]
pub async fn export_diagnostics(
    window: WebviewWindow,
    diagnostics: State<'_, Diagnostics>,
    token: String,
) -> Result<DiagnosticExportDto, String> {
    authorize(&window)?;
    let preview = diagnostics
        .preview
        .lock()
        .map_err(|e| e.to_string())?
        .1
        .clone()
        .filter(|p| p.token == token)
        .ok_or("诊断预览已更新，请重新预览后导出")?;
    let directory = window
        .app_handle()
        .path()
        .download_dir()
        .map_err(|_| "下载目录不可用")?;
    let path = tauri::async_runtime::spawn_blocking(move || {
        pinmeter_platform::diagnostics::export(
            &directory,
            &preview.content,
            preview.generated_at_ms as u64,
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(DiagnosticExportDto {
        saved: true,
        path: Some(path.to_string_lossy().into_owned()),
    })
}
