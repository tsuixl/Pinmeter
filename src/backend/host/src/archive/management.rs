use super::{Archives, DAY, HOUR, MONTH, resolution};
use crate::runtime::Runtime;
use pinmeter_core::ports::Clock;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};
use ts_rs::TS;

#[derive(Clone, Serialize, TS)]
pub struct HistoryStorageDto {
    pub basic_bytes: Option<String>,
    pub applications_bytes: Option<String>,
    pub basic_limit_bytes: String,
    pub applications_limit_bytes: String,
    pub detail: String,
}
#[derive(Clone, Serialize, TS)]
pub struct HistoryExportDto {
    pub path: String,
    pub from_ms: f64,
    pub through_ms: f64,
    pub resolution_ms: f64,
}

#[tauri::command]
pub async fn get_history_storage(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<HistoryStorageDto, String> {
    crate::commands::authorize(&window)?;
    let basic = runtime.archives.lock().map_err(|e| e.to_string())?.clone();
    let apps = runtime
        .app_history
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut errors = vec![];
        let basic_bytes = basic
            .ok_or_else(|| "基础历史目录不可用".to_string())
            .and_then(|s| s.storage_bytes())
            .map_err(|error| errors.push(error))
            .ok();
        let applications_bytes = apps
            .ok_or_else(|| "应用历史目录不可用".to_string())
            .and_then(|s| s.storage_bytes())
            .map_err(|error| errors.push(error))
            .ok();
        HistoryStorageDto {
            basic_bytes: basic_bytes.map(|bytes| bytes.to_string()),
            applications_bytes: applications_bytes.map(|bytes| bytes.to_string()),
            basic_limit_bytes: pinmeter_core::archive::MAX_FILE_BYTES.to_string(),
            applications_limit_bytes: pinmeter_core::app_history::MAX_FILE_BYTES.to_string(),
            detail: errors.join("；"),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    scope: String,
) -> Result<(), String> {
    crate::commands::authorize(&window)?;
    let basic = runtime.archives.lock().map_err(|e| e.to_string())?.clone();
    let apps = runtime
        .app_history
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let cutoff = pinmeter_platform::shared::SystemClock::default().wall_ms();
    tauri::async_runtime::spawn_blocking(move || match scope.as_str() {
        "basic" => basic.ok_or("基础历史目录不可用")?.clear(cutoff),
        "applications" => apps.ok_or("应用历史目录不可用")?.clear(cutoff),
        _ => Err("未知历史类型".into()),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_history(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    scope: String,
    from_ms: f64,
    through_ms: f64,
    include_paths: bool,
) -> Result<HistoryExportDto, String> {
    crate::commands::authorize(&window)?;
    let now = pinmeter_platform::shared::SystemClock::default().wall_ms();
    if !from_ms.is_finite()
        || !through_ms.is_finite()
        || from_ms.fract() != 0.
        || through_ms.fract() != 0.
        || from_ms < now.saturating_sub(MONTH + HOUR) as f64
        || from_ms >= through_ms
        || through_ms > now as f64
    {
        return Err("导出时间范围无效，请重新选择最近30天内的区间".into());
    }
    let basic = runtime.archives.lock().map_err(|e| e.to_string())?.clone();
    let apps = runtime
        .app_history
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let directory = window
        .app_handle()
        .path()
        .download_dir()
        .map_err(|_| "下载目录不可用")?;
    let from = from_ms as u64;
    let through = through_ms as u64;
    // The selected UI range may include its first partial bucket.
    let span = through - from;
    let resolution_ms = resolution(if span <= DAY + pinmeter_core::archive::MINUTE {
        DAY
    } else if span <= pinmeter_core::archive::WEEK + pinmeter_core::archive::QUARTER_HOUR {
        pinmeter_core::archive::WEEK
    } else {
        MONTH
    });
    let path = tauri::async_runtime::spawn_blocking(move || {
        let content = match scope.as_str() {
            "basic" => {
                basic
                    .ok_or("基础历史目录不可用")?
                    .export_rows(from, through, resolution_ms)?
            }
            "applications" => apps.ok_or("应用历史目录不可用")?.export_rows(
                from,
                through,
                resolution_ms,
                include_paths,
            )?,
            _ => return Err("未知历史类型".into()),
        };
        pinmeter_platform::archive::export_csv(&directory, &scope, &content, now)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(HistoryExportDto {
        path: path.to_string_lossy().into_owned(),
        from_ms,
        through_ms,
        resolution_ms: resolution_ms as f64,
    })
}

/// Spreadsheet formulas are not activated even when a filename starts with =+-@.
pub(crate) fn csv_cell(value: &str) -> String {
    let formula = value.trim_start().starts_with(['=', '+', '-', '@'])
        || value.starts_with(['\t', '\r', '\n']);
    format!(
        "\"{}{}\"",
        if formula { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}
fn optional(value: Option<f64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}
impl Archives {
    fn export_rows(&self, from: u64, through: u64, resolution_ms: u64) -> Result<String, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let mut csv = "bucket_start_unix_ms,resolution_ms,cpu_average_percent,cpu_peak_percent,cpu_peak_unix_ms,cpu_coverage_ms,memory_average_percent,memory_peak_percent,memory_peak_unix_ms,memory_coverage_ms,received_bytes,transmitted_bytes,network_coverage_ms,download_peak_bytes_per_second,download_peak_unix_ms,upload_peak_bytes_per_second,upload_peak_unix_ms,networks\r\n".to_string();
        for (&at, bucket) in data
            .archive
            .layer(resolution_ms)
            .range(from / resolution_ms * resolution_ms..through)
        {
            let fields = [
                at.to_string(),
                resolution_ms.to_string(),
                optional(bucket.cpu.average()),
                optional(bucket.cpu.max),
                bucket
                    .cpu
                    .max_at_ms
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                bucket.cpu.covered_ms.to_string(),
                optional(bucket.memory.average()),
                optional(bucket.memory.max),
                bucket
                    .memory
                    .max_at_ms
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                bucket.memory.covered_ms.to_string(),
                if bucket.network_ms > 0 {
                    bucket.received.to_string()
                } else {
                    String::new()
                },
                if bucket.network_ms > 0 {
                    bucket.transmitted.to_string()
                } else {
                    String::new()
                },
                bucket.network_ms.to_string(),
                optional(bucket.download_peak.as_ref().map(|p| p.value)),
                bucket
                    .download_peak
                    .as_ref()
                    .map(|p| p.at_ms.to_string())
                    .unwrap_or_default(),
                optional(bucket.upload_peak.as_ref().map(|p| p.value)),
                bucket
                    .upload_peak
                    .as_ref()
                    .map(|p| p.at_ms.to_string())
                    .unwrap_or_default(),
                csv_cell(
                    &bucket
                        .networks
                        .iter()
                        .map(|n| n.name.clone())
                        .collect::<Vec<_>>()
                        .join(";"),
                ),
            ];
            csv.push_str(&fields.join(","));
            csv.push_str("\r\n");
        }
        Ok(csv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_escapes_text_and_disables_spreadsheet_formulas() {
        assert_eq!(csv_cell("=1+1"), "\"'=1+1\"");
        assert_eq!(csv_cell("  @cmd"), "\"'  @cmd\"");
        assert_eq!(csv_cell("a,\"b\""), "\"a,\"\"b\"\"\"");
        assert_eq!(optional(None), "");
    }
}
