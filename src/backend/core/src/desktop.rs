//! Platform-neutral desktop preferences, projections and operation intents.
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct StartupVisibility {
    completed: bool,
    reveal_requested: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum StartupAction {
    Show,
    Tray,
    KeepCurrent,
}
impl StartupVisibility {
    pub fn reveal(&mut self) {
        self.reveal_requested = true;
    }
    pub fn complete(&mut self, start_in_tray: bool, tray_ready: bool) -> StartupAction {
        if self.completed {
            return StartupAction::KeepCurrent;
        }
        self.completed = true;
        if start_in_tray && tray_ready && !self.reveal_requested {
            StartupAction::Tray
        } else {
            StartupAction::Show
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TaskbarSettings {
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
impl Default for TaskbarSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            hidden: false,
            layout: "double".into(),
            cpu: true,
            gpu: true,
            memory: true,
            network: true,
            cpu_temperature: true,
            gpu_temperature: true,
            gpu_id: None,
            order: vec![
                "network".into(),
                "cpu".into(),
                "gpu".into(),
                "memory".into(),
            ],
        }
    }
}
impl TaskbarSettings {
    pub fn group_enabled(&self, key: &str) -> bool {
        match key {
            "network" => self.network,
            "cpu" => self.cpu || self.cpu_temperature,
            "gpu" => self.gpu || self.gpu_temperature,
            "memory" => self.memory,
            _ => false,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.layout.as_str(), "double" | "single") {
            return Err("无效的任务栏布局".into());
        }
        let mut order = self.order.clone();
        order.sort();
        if order != ["cpu", "gpu", "memory", "network"] {
            return Err("任务栏排序必须包含且只包含四类指标".into());
        }
        if self.enabled && !self.order.iter().any(|k| self.group_enabled(k)) {
            return Err("任务栏至少需要保留一个指标".into());
        }
        if self
            .gpu_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 256)
        {
            return Err("无效的显卡标识".into());
        }
        Ok(())
    }
}
impl<'de> Deserialize<'de> for TaskbarSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Stored {
            enabled: bool,
            hidden: bool,
            layout: String,
            cpu: bool,
            gpu: bool,
            memory: bool,
            network: bool,
            cpu_temperature: Option<bool>,
            gpu_temperature: Option<bool>,
            gpu_id: Option<String>,
            order: Vec<String>,
        }
        impl Default for Stored {
            fn default() -> Self {
                let d = TaskbarSettings::default();
                Self {
                    enabled: d.enabled,
                    hidden: d.hidden,
                    layout: d.layout,
                    cpu: d.cpu,
                    gpu: d.gpu,
                    memory: d.memory,
                    network: d.network,
                    cpu_temperature: None,
                    gpu_temperature: None,
                    gpu_id: d.gpu_id,
                    order: d.order,
                }
            }
        }
        let d = Stored::deserialize(deserializer)?;
        Ok(Self {
            enabled: d.enabled,
            hidden: d.hidden,
            layout: d.layout,
            cpu: d.cpu,
            gpu: d.gpu,
            memory: d.memory,
            network: d.network,
            cpu_temperature: d.cpu_temperature.unwrap_or(d.cpu),
            gpu_temperature: d.gpu_temperature.unwrap_or(d.gpu),
            gpu_id: d.gpu_id,
            order: d.order,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DesktopStatus {
    pub supported: bool,
    pub stage: String,
    pub detail: String,
    pub revision: u64,
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    #[test]
    fn startup_hides_only_once_with_a_working_tray() {
        let mut state = StartupVisibility::default();
        assert_eq!(state.complete(true, true), StartupAction::Tray);
        state.reveal();
        assert_eq!(state.complete(true, true), StartupAction::KeepCurrent);
    }
    #[test]
    fn missing_tray_and_explicit_recovery_keep_the_window_available() {
        assert_eq!(
            StartupVisibility::default().complete(true, false),
            StartupAction::Show
        );
        assert_eq!(
            StartupVisibility::default().complete(false, true),
            StartupAction::Show
        );
        let mut state = StartupVisibility::default();
        state.reveal();
        assert_eq!(state.complete(true, true), StartupAction::Show);
    }
}

#[cfg(test)]
mod display_settings_tests {
    use super::*;
    #[test]
    fn old_groups_keep_their_temperature_visibility_and_invalid_settings_are_rejected() {
        let old: TaskbarSettings = serde_json::from_str(r#"{"cpu":false,"gpu":true}"#).unwrap();
        assert!(!old.cpu_temperature);
        assert!(old.gpu_temperature);
        assert!(old.network);
        let mut invalid = TaskbarSettings {
            enabled: true,
            network: false,
            cpu: false,
            cpu_temperature: false,
            gpu: false,
            gpu_temperature: false,
            memory: false,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
        invalid.cpu_temperature = true;
        assert!(invalid.validate().is_ok());
        invalid.order = vec!["cpu".into(); 4];
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn native_groups_match_shared_preview_cases() {
        #[derive(Deserialize)]
        struct Case {
            settings: TaskbarSettings,
            expected: Vec<Vec<usize>>,
            compact: Vec<Vec<usize>>,
        }
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../../../shared/fixtures/taskbar-groups.json"))
                .unwrap();
        for case in cases {
            let summary = DesktopSummary {
                session: "test".into(),
                cursor: 0,
                network_id: None,
                gpu_id: None,
                revision: 0,
                settings: case.settings,
                network: String::new(),
                readings: ["network", "network", "cpu", "memory", "gpu"]
                    .into_iter()
                    .map(|key| SummaryReading {
                        show_value: true,
                        valid_at_ms: None,
                        key,
                        label: key,
                        text: String::new(),
                        unit: String::new(),
                        detail: String::new(),
                        normal: false,
                        temperature: None,
                    })
                    .collect(),
            };
            let full = summary_groups(&summary, false);
            let actual = if summary.settings.layout == "single" {
                full.into_iter().flatten().map(|i| vec![i]).collect()
            } else {
                full
            };
            assert_eq!(actual, case.expected);
            assert_eq!(summary_groups(&summary, true), case.compact);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryReading {
    pub show_value: bool,
    pub valid_at_ms: Option<u64>,
    pub key: &'static str,
    pub label: &'static str,
    pub text: String,
    pub unit: String,
    pub detail: String,
    pub normal: bool,
    pub temperature: Option<SummaryTemperature>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryTemperature {
    pub text: String,
    pub normal: bool,
    pub detail: String,
    pub valid_at_ms: Option<u64>,
    pub alternative_sensor: bool,
}
impl SummaryTemperature {
    pub fn formatted(&self) -> String {
        format!(
            "({}°C{})",
            self.text,
            if self.alternative_sensor { "*" } else { "" }
        )
    }
}
impl SummaryReading {
    pub fn display(&self) -> String {
        let temperature = self
            .temperature
            .as_ref()
            .map(|t| format!(" {}", t.formatted()))
            .unwrap_or_default();
        if self.show_value {
            format!("{} {}{}{}", self.label, self.text, self.unit, temperature)
        } else {
            format!("{}{}", self.label, temperature)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSummary {
    pub session: String,
    pub cursor: u64,
    pub network_id: Option<String>,
    pub gpu_id: Option<String>,
    pub revision: u64,
    pub settings: TaskbarSettings,
    pub network: String,
    pub readings: Vec<SummaryReading>,
}
pub fn summary_groups(summary: &DesktopSummary, compact: bool) -> Vec<Vec<usize>> {
    let mut groups = vec![];
    let mut pending = vec![];
    for key in &summary.settings.order {
        if !summary.settings.group_enabled(key) {
            continue;
        }
        let indices: Vec<_> = summary
            .readings
            .iter()
            .enumerate()
            .filter_map(|(i, r)| (r.key == key).then_some(i))
            .collect();
        if key == "network" {
            if !pending.is_empty() {
                groups.push(std::mem::take(&mut pending));
            }
            if !indices.is_empty() {
                groups.push(indices);
            }
        } else {
            for i in indices {
                pending.push(i);
                if pending.len() == 2 {
                    groups.push(std::mem::take(&mut pending));
                }
            }
        }
        if compact {
            break;
        }
    }
    if !pending.is_empty() {
        groups.push(pending);
    }
    groups
}
#[derive(Clone, Copy, Debug)]
pub enum DesktopIntent {
    Open(&'static str),
    ToggleReadings,
    Exit,
}

/// Choose a gap from actual occupied intervals, never from fixed taskbar offsets.
pub fn rightmost_gap(
    left: i32,
    right: i32,
    occupied: &[(i32, i32)],
    width: i32,
    margin: i32,
) -> Option<i32> {
    if width <= 0 || margin < 0 || right <= left {
        return None;
    }
    let mut intervals: Vec<_> = occupied
        .iter()
        .copied()
        .filter(|(a, b)| b > a && *b > left && *a < right)
        .collect();
    intervals.sort_unstable();
    let mut cursor = left + margin;
    let mut result = None;
    for (start, end) in intervals {
        if start - margin - cursor >= width {
            result = Some(start - margin - width);
        }
        cursor = cursor.max(end.saturating_add(margin));
    }
    if right - margin - cursor >= width {
        result = Some(right - margin - width);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_real_gaps_with_overlaps_and_negative_monitor_coordinates() {
        assert_eq!(
            rightmost_gap(
                -1920,
                0,
                &[(-1920, -1600), (-1700, -1500), (-240, 0)],
                220,
                8
            ),
            Some(-468)
        );
        assert_eq!(rightmost_gap(0, 400, &[(0, 220), (230, 400)], 100, 4), None);
        assert_eq!(
            rightmost_gap(0, 400, &[(0, 220), (350, 400)], 100, 4),
            Some(246)
        );
        assert_eq!(rightmost_gap(0, 400, &[(0, 400)], 10, 0), None);
    }
}
