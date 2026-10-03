use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct GpuDeviceDto {
    pub id: String,
    pub name: String,
    pub readings: std::collections::BTreeMap<String, ReadingDto>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct GpuSnapshotDto {
    pub devices: Vec<GpuDeviceDto>,
    pub status: ReadingStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    Normal,
    Warming,
    Unsupported,
    PermissionDenied,
    Failed,
    Stale,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ReadingDto {
    pub value: Option<f64>,
    pub text: String,
    pub unit: String,
    pub status: ReadingStatus,
    pub valid_at_ms: Option<f64>,
    pub source: String,
    pub semantic: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct FrameDto {
    pub gpus: Vec<GpuDeviceDto>,
    pub cursor: String,
    pub at_ms: f64,
    pub elapsed_ms: f64,
    pub generation: String,
    pub network_generation: String,
    pub network_id: Option<String>,
    pub cpu: ReadingDto,
    pub cpu_temperature: ReadingDto,
    pub memory: ReadingDto,
    pub download: ReadingDto,
    pub upload: ReadingDto,
    pub memory_used: String,
    pub memory_total: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SettingsDto {
    #[serde(default)]
    pub start_in_tray: bool,
    pub autostart: bool,
    pub close_action: String,
    pub taskbar: TaskbarSettingsDto,
    pub revision: String,
    pub theme: String,
    pub interval_ms: u32,
    pub release_network_on_exit: bool,
    pub network_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TaskbarSettingsDto {
    pub enabled: bool,
    pub hidden: bool,
    pub layout: String,
    pub cpu: bool,
    pub gpu: bool,
    pub memory: bool,
    pub network: bool,
    pub cpu_temperature: bool,
    pub gpu_temperature: bool,
    pub gpu_id: Option<String>,
    pub order: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct DesktopStatusDto {
    pub supported: bool,
    pub stage: String,
    pub detail: String,
    pub revision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AutostartStatusDto {
    pub available: bool,
    pub enabled: Option<bool>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct InterfaceDto {
    pub id: String,
    pub name: String,
    pub up: bool,
    pub physical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct CapabilityDto {
    pub metric: String,
    pub available: bool,
    pub status: ReadingStatus,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ProcessorDto {
    pub id: String,
    pub usage: ReadingDto,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct CpuProcessorsDto {
    pub processors: Vec<ProcessorDto>,
    pub status: ReadingStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AppTrafficDto {
    pub download: Option<f64>,
    pub upload: Option<f64>,
    pub received: String,
    pub sent: String,
    pub download_share: Option<f64>,
    pub upload_share: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AppProcessDto {
    pub id: String,
    pub pid: u32,
    pub observed: bool,
    pub traffic: AppTrafficDto,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AppNetworkRowDto {
    pub icon: Option<String>,
    pub id: String,
    pub name: String,
    pub path: String,
    pub traffic: AppTrafficDto,
    pub processes: Vec<AppProcessDto>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AppNetworkDto {
    pub session: String,
    pub status: String,
    pub detail: String,
    pub running: bool,
    pub incomplete: bool,
    pub limited: bool,
    pub apps: Vec<AppNetworkRowDto>,
    pub unknown: AppTrafficDto,
    pub other: AppTrafficDto,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MonitorStateDto {
    pub autostart: AutostartStatusDto,
    pub desktop: DesktopStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub network_control: Option<crate::network_control::NetworkControlDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ip: Option<crate::ip::dto::IpStateDto>,
    pub cpu_model: Option<String>,
    pub gpu: GpuSnapshotDto,
    pub app_network: AppNetworkDto,
    pub protocol_version: u32,
    pub session_id: String,
    pub platform: String,
    pub settings: SettingsDto,
    pub applied_settings_revision: String,
    pub capabilities_version: String,
    pub capabilities: Vec<CapabilityDto>,
    pub interfaces: Vec<InterfaceDto>,
    pub selected_interface: Option<InterfaceDto>,
    pub frame: Option<FrameDto>,
    pub cpu_processors: CpuProcessorsDto,
    pub cpu_temperature: ReadingDto,
    pub history: Vec<FrameDto>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MonitorBatchDto {
    pub kind: String,
    pub subscription_id: String,
    pub delivery_seq: String,
    pub history_from: String,
    pub history_to: String,
    pub requires_history_sync: bool,
    pub state: MonitorStateDto,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HistoryDto {
    pub session_id: String,
    pub frames: Vec<FrameDto>,
    pub retained_from: String,
    pub through: String,
    pub missing: bool,
}

pub fn typescript() -> String {
    let config = ts_rs::Config::default();
    let declarations = [
        crate::disk::DiskPointDto::decl(&config),
        crate::disk::DiskFrameDto::decl(&config),
        crate::disk::DiskSnapshotDto::decl(&config),
        crate::diagnostics::DiagnosticPreviewDto::decl(&config),
        crate::diagnostics::DiagnosticExportDto::decl(&config),
        crate::updates::dto::ReleaseSectionDto::decl(&config),
        crate::updates::dto::ReleaseNotesDto::decl(&config),
        crate::updates::dto::UpdateSnapshotDto::decl(&config),
        crate::hardware::HardwareItemDto::decl(&config),
        crate::hardware::HardwareSectionDto::decl(&config),
        crate::hardware::HardwareInfoDto::decl(&config),
        AutostartStatusDto::decl(&config),
        TaskbarSettingsDto::decl(&config),
        DesktopStatusDto::decl(&config),
        crate::network_control::NetworkRuleDto::decl(&config),
        crate::network_control::NetworkControlDto::decl(&config),
        GpuDeviceDto::decl(&config),
        GpuSnapshotDto::decl(&config),
        AppTrafficDto::decl(&config),
        AppProcessDto::decl(&config),
        AppNetworkRowDto::decl(&config),
        AppNetworkDto::decl(&config),
        ReadingStatus::decl(&config),
        ReadingDto::decl(&config),
        ProcessorDto::decl(&config),
        CpuProcessorsDto::decl(&config),
        FrameDto::decl(&config),
        SettingsDto::decl(&config),
        crate::exit::ExitStatusDto::decl(&config),
        InterfaceDto::decl(&config),
        CapabilityDto::decl(&config),
        MonitorStateDto::decl(&config),
        MonitorBatchDto::decl(&config),
        HistoryDto::decl(&config),
    ];
    format!(
        "// Generated from host/src/contracts. Run tools/dev.ps1 contracts.\n{}\n",
        crate::ip::dto::declarations(&config)
            .into_iter()
            .chain(declarations)
            .map(|d| format!("export {d}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
