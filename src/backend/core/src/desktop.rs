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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TaskbarSettings {
    pub enabled: bool,
    pub hidden: bool,
    pub layout: String,
    pub cpu: bool,
    pub gpu: bool,
    pub memory: bool,
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
        }
    }
}
impl TaskbarSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.layout.as_str(), "double" | "single") {
            return Err("无效的任务栏布局".into());
        }
        Ok(())
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryReading {
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
        format!("{} {}{}{}", self.label, self.text, self.unit, temperature)
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
