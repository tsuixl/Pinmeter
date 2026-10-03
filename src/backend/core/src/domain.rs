use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Failure {
    pub status: Status,
    pub reason: String,
}
impl Failure {
    pub fn new(status: Status, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Observation<T> {
    pub result: Result<T, Failure>,
    pub source: &'static str,
    pub semantic: &'static str,
    pub mono_ms: u64,
    pub wall_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkInterface {
    pub id: String,
    pub name: String,
    pub up: bool,
    pub physical: bool,
}
#[derive(Clone, Debug)]
pub struct NetworkCounters {
    pub interface: NetworkInterface,
    pub received: u64,
    pub transmitted: u64,
}
#[derive(Clone, Debug)]
pub struct Memory {
    pub used: u64,
    pub total: u64,
}
#[derive(Clone, Debug)]
pub struct RawSample {
    pub cpu: Observation<f64>,
    pub cpu_processors: Observation<Vec<ProcessorSample>>,
    pub memory: Observation<Memory>,
    pub network: Observation<Vec<NetworkCounters>>,
}

#[derive(Clone, Debug)]
pub struct ProcessorSample {
    pub id: String,
    pub usage: Result<f64, Failure>,
}

#[derive(Clone, Debug)]
pub struct ProcessorReading {
    pub id: String,
    pub usage: Reading,
}

#[derive(Clone, Debug)]
pub struct CpuProcessors {
    pub processors: Vec<ProcessorReading>,
    pub status: Status,
    pub detail: String,
    pub sampled_mono_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub value: Option<f64>,
    pub status: Status,
    pub valid_at_ms: Option<u64>,
    pub valid_mono_ms: Option<u64>,
    pub source: &'static str,
    pub semantic: &'static str,
    pub detail: String,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub gpus: Vec<crate::gpu::GpuDevice>,
    pub cursor: u64,
    pub at_ms: u64,
    pub elapsed_ms: u64,
    pub generation: u64,
    pub network_generation: u64,
    pub network_id: Option<String>,
    pub cpu: Reading,
    pub cpu_temperature: Reading,
    pub memory: Reading,
    pub download: Reading,
    pub upload: Reading,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
}

#[derive(Default)]
pub struct RateBaseline {
    pub delta: Option<TrafficDelta>,
    previous: Option<(String, u64, u64, u64)>,
    pub generation: u64,
}
impl RateBaseline {
    pub fn reset(&mut self) {
        self.delta = None;
        self.previous = None;
        self.generation += 1;
    }
    pub fn rates(
        &mut self,
        counters: &NetworkCounters,
        now: u64,
        max_gap_ms: u64,
    ) -> Result<(f64, f64), Failure> {
        self.delta = None;
        let next = (
            counters.interface.id.clone(),
            counters.received,
            counters.transmitted,
            now,
        );
        let previous = self.previous.replace(next);
        if let Some((id, received, transmitted, at)) = previous {
            if id == counters.interface.id
                && now > at
                && now - at <= max_gap_ms
                && counters.received >= received
                && counters.transmitted >= transmitted
            {
                let seconds = (now - at) as f64 / 1000.0;
                self.delta = Some(TrafficDelta {
                    received: counters.received - received,
                    transmitted: counters.transmitted - transmitted,
                    elapsed_ms: now - at,
                });
                return Ok((
                    (counters.received - received) as f64 / seconds,
                    (counters.transmitted - transmitted) as f64 / seconds,
                ));
            }
            self.generation += 1;
        }
        Err(Failure::new(Status::Warming, "正在建立网络速率基线"))
    }
}

#[derive(Clone, Debug)]
pub struct TrafficDelta {
    pub received: u64,
    pub transmitted: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Normal,
    Warming,
    Unsupported,
    PermissionDenied,
    Failed,
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub record_app_traffic_on_start: bool,
    #[serde(default)]
    pub start_in_tray: bool,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "ask_on_close")]
    pub close_action: String,
    #[serde(default)]
    pub taskbar: crate::desktop::TaskbarSettings,
    pub schema_version: u32,
    pub revision: u64,
    pub theme: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "crate::fonts::default_font_style")]
    pub font_style: String,
    pub interval_ms: u64,
    pub network_id: Option<String>,
    #[serde(default = "release_network_by_default")]
    pub release_network_on_exit: bool,
}
fn ask_on_close() -> String {
    "ask".into()
}
pub fn default_font_family() -> String {
    "harmonyos_sans_sc".into()
}
fn release_network_by_default() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            record_app_traffic_on_start: false,
            autostart: false,
            start_in_tray: false,
            close_action: ask_on_close(),
            schema_version: 1,
            taskbar: Default::default(),
            revision: 0,
            theme: "system".into(),
            font_family: default_font_family(),
            font_style: crate::fonts::default_font_style(),
            interval_ms: 1000,
            network_id: None,
            release_network_on_exit: true,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.taskbar.validate()?;
        if !matches!(self.close_action.as_str(), "ask" | "minimize" | "exit") {
            return Err("无效的关闭行为".into());
        }
        if self.schema_version != 1 {
            return Err("不支持的配置版本".into());
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            return Err("无效的主题".into());
        }
        if !crate::fonts::valid_family(&self.font_family) {
            return Err("无效的界面字体".into());
        }
        if crate::fonts::style_axes(&self.font_style).is_none() {
            return Err("无效的字体样式".into());
        }
        if !matches!(self.interval_ms, 1000 | 2000 | 5000) {
            return Err("采样间隔必须为 1、2 或 5 秒".into());
        }
        if self
            .network_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 256)
        {
            return Err("无效的网卡标识".into());
        }
        Ok(())
    }
}
