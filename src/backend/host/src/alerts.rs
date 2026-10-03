use crate::{commands::authorize, runtime::Runtime};
use pinmeter_core::alerts::{AlertRule, AlertsConfig, QuietHours};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AlertRuleDto {
    pub enabled: bool,
    pub threshold_percent: u8,
    pub duration_seconds: u16,
    pub cooldown_seconds: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct QuietHoursDto {
    pub enabled: bool,
    pub start_minute: u16,
    pub end_minute: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AlertsConfigDto {
    pub cpu: AlertRuleDto,
    pub memory: AlertRuleDto,
    pub quiet: QuietHoursDto,
}
impl Default for AlertsConfigDto {
    fn default() -> Self {
        AlertsConfig::default().into()
    }
}
impl From<AlertRule> for AlertRuleDto {
    fn from(rule: AlertRule) -> Self {
        Self {
            enabled: rule.enabled,
            threshold_percent: rule.threshold_percent,
            duration_seconds: rule.duration_seconds,
            cooldown_seconds: rule.cooldown_seconds,
        }
    }
}
impl From<AlertRuleDto> for AlertRule {
    fn from(rule: AlertRuleDto) -> Self {
        Self {
            enabled: rule.enabled,
            threshold_percent: rule.threshold_percent,
            duration_seconds: rule.duration_seconds,
            cooldown_seconds: rule.cooldown_seconds,
        }
    }
}
impl From<AlertsConfig> for AlertsConfigDto {
    fn from(value: AlertsConfig) -> Self {
        Self {
            cpu: value.cpu.into(),
            memory: value.memory.into(),
            quiet: QuietHoursDto {
                enabled: value.quiet.enabled,
                start_minute: value.quiet.start_minute,
                end_minute: value.quiet.end_minute,
            },
        }
    }
}
impl From<AlertsConfigDto> for AlertsConfig {
    fn from(value: AlertsConfigDto) -> Self {
        Self {
            cpu: value.cpu.into(),
            memory: value.memory.into(),
            quiet: QuietHours {
                enabled: value.quiet.enabled,
                start_minute: value.quiet.start_minute,
                end_minute: value.quiet.end_minute,
            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AlertEventDto {
    pub id: String,
    #[ts(type = "'cpu' | 'memory'")]
    pub metric: String,
    pub at_ms: f64,
    pub value: f64,
    pub threshold_percent: u8,
    pub duration_seconds: u16,
    pub acknowledged: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AlertsSnapshotDto {
    pub config: AlertsConfigDto,
    pub settings_revision: String,
    pub revision: String,
    pub events: Vec<AlertEventDto>,
    pub unread_count: u32,
    pub local_time_available: bool,
    pub quiet_now: bool,
    pub delivery_detail: String,
}

impl Runtime {
    /// Consumes accepted core frames only; there is no separate sampling or worker.
    pub fn accept_alert_frame(&self) {
        let state = self.inner.lock().unwrap();
        let monitor = &state.monitor;
        if let Some(frame) = monitor.history.back() {
            self.alerts.lock().unwrap().accept(
                &monitor.settings.alerts,
                frame,
                pinmeter_platform::alerts::local_minute(),
                monitor.settings.interval_ms * 3,
            );
        }
    }
    fn alert_snapshot(&self) -> Result<AlertsSnapshotDto, String> {
        let state = self.inner.lock().map_err(|e| e.to_string())?;
        let alerts = self.alerts.lock().map_err(|e| e.to_string())?;
        let minute = pinmeter_platform::alerts::local_minute();
        Ok(AlertsSnapshotDto {
            config: state.monitor.settings.alerts.clone().into(),
            settings_revision: state.monitor.settings.revision.to_string(),
            revision: alerts.revision().to_string(),
            events: alerts.events().iter().map(|event| AlertEventDto {
                id: event.id.to_string(),
                metric: event.metric.key().into(),
                at_ms: event.at_ms as f64,
                value: event.value,
                threshold_percent: event.threshold_percent,
                duration_seconds: event.duration_seconds,
                acknowledged: event.acknowledged,
            }).collect(),
            unread_count: alerts.unread_count() as u32,
            local_time_available: minute.is_some(),
            quiet_now: state.monitor.settings.alerts.quiet.contains(minute),
            delivery_detail: "应用内提醒和托盘提示，不发送系统弹窗。事件仅在本次运行保留，最多 20 条；退出后清空。".into(),
        })
    }
}

#[tauri::command]
pub fn get_alerts_snapshot(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<AlertsSnapshotDto, String> {
    authorize(&window)?;
    runtime.alert_snapshot()
}
#[tauri::command]
pub async fn update_alerts(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    config: AlertsConfigDto,
    expected_revision: String,
) -> Result<AlertsSnapshotDto, String> {
    authorize(&window)?;
    let expected = expected_revision
        .parse::<u64>()
        .map_err(|_| "无效的设置版本")?;
    let config: AlertsConfig = config.into();
    config.validate()?;
    if config.quiet.enabled && pinmeter_platform::alerts::local_minute().is_none() {
        return Err("此平台尚不支持本地静默时段；请关闭静默设置后保存".into());
    }
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime.save_settings("idle", |current| {
            let mut next = current.clone();
            next.alerts = config;
            Ok((next, expected))
        })?;
        runtime.alert_snapshot()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn acknowledge_alert(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    id: String,
) -> Result<AlertsSnapshotDto, String> {
    authorize(&window)?;
    let id = id.parse().map_err(|_| "无效的提醒编号")?;
    runtime
        .alerts
        .lock()
        .map_err(|e| e.to_string())?
        .acknowledge(id);
    runtime.alert_snapshot()
}
#[tauri::command]
pub fn clear_alert_events(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    through_id: String,
) -> Result<AlertsSnapshotDto, String> {
    authorize(&window)?;
    let id = through_id.parse().map_err(|_| "无效的提醒编号")?;
    runtime
        .alerts
        .lock()
        .map_err(|e| e.to_string())?
        .clear_through(id);
    runtime.alert_snapshot()
}
