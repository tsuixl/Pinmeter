use crate::{application::Monitor, domain::Status};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticSystem {
    pub os: String,
    pub architecture: String,
    pub version: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct DiagnosticReading {
    pub metric: String,
    pub status: String,
    pub value: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct DiagnosticSettings {
    pub theme: String,
    pub interval_ms: u64,
    pub autostart: bool,
    pub start_in_tray: bool,
    pub release_network_on_exit: bool,
    pub taskbar_enabled: bool,
    pub network_selection: String,
}
#[derive(Debug, Serialize)]
pub struct DiagnosticReport {
    pub schema_version: u32,
    pub application_version: String,
    pub generated_at_ms: u64,
    pub system: DiagnosticSystem,
    pub settings: DiagnosticSettings,
    pub history_frames: usize,
    pub gpu_count: usize,
    pub has_startup_diagnostic: bool,
    pub readings: Vec<DiagnosticReading>,
}
fn status(value: &Status) -> String {
    match value {
        Status::Normal => "normal",
        Status::Warming => "warming",
        Status::Unsupported => "unsupported",
        Status::PermissionDenied => "permission_denied",
        Status::Failed => "failed",
        Status::Stale => "stale",
    }
    .into()
}
/// A field allowlist: raw errors, identifiers, paths, IP data and process rows never enter the report.
pub fn capture(
    monitor: &Monitor,
    system: DiagnosticSystem,
    version: &str,
    wall_ms: u64,
    mono_ms: u64,
) -> DiagnosticReport {
    let current = monitor.latest_at(mono_ms);
    let mut readings = vec![];
    for (name, value) in [
        ("cpu", current.as_ref().map(|f| &f.cpu)),
        ("memory", current.as_ref().map(|f| &f.memory)),
        ("download", current.as_ref().map(|f| &f.download)),
        ("upload", current.as_ref().map(|f| &f.upload)),
    ] {
        readings.push(DiagnosticReading {
            metric: name.into(),
            status: status(value.map_or(&Status::Warming, |r| &r.status)),
            value: value.and_then(|r| r.value),
        });
    }
    readings.push(DiagnosticReading {
        metric: "cpu_temperature".into(),
        status: status(&monitor.cpu_temperature_at(mono_ms).status),
        value: monitor.cpu_temperature_at(mono_ms).value,
    });
    let gpu = monitor.gpu.at(mono_ms);
    for (i, device) in gpu.devices.iter().enumerate() {
        for (metric, reading) in &device.readings {
            readings.push(DiagnosticReading {
                metric: format!("gpu_{}.{}", i + 1, metric.key()),
                status: status(&reading.status),
                value: reading.value,
            });
        }
    }
    DiagnosticReport {
        schema_version: 1,
        application_version: version.into(),
        generated_at_ms: wall_ms,
        system,
        settings: DiagnosticSettings {
            theme: monitor.settings.theme.clone(),
            interval_ms: monitor.settings.interval_ms,
            autostart: monitor.settings.autostart,
            start_in_tray: monitor.settings.start_in_tray,
            release_network_on_exit: monitor.settings.release_network_on_exit,
            taskbar_enabled: monitor.settings.taskbar.enabled,
            network_selection: if monitor.settings.network_id.is_some() {
                "manual"
            } else {
                "auto"
            }
            .into(),
        },
        history_frames: monitor.history.len(),
        gpu_count: gpu.devices.len(),
        has_startup_diagnostic: monitor.diagnostic.is_some(),
        readings,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_preserves_states_without_exporting_sensitive_source_fields() {
        let settings = crate::domain::Settings {
            network_id: Some("private-adapter-id".into()),
            ..Default::default()
        };
        let mut monitor = Monitor::new(
            settings,
            Some("C:\\Users\\private-user\\config token=private-token https://192.0.2.1".into()),
        );
        monitor.cpu_model = Some("private-device-name".into());
        let report = capture(
            &monitor,
            DiagnosticSystem {
                os: "windows".into(),
                architecture: "x86_64".into(),
                version: None,
            },
            "0.1.3",
            1000,
            1000,
        );
        let json = serde_json::to_string(&report).unwrap();
        for secret in [
            "private-adapter-id",
            "private-user",
            "private-token",
            "192.0.2.1",
            "private-device-name",
        ] {
            assert!(!json.contains(secret));
        }
        assert!(report.has_startup_diagnostic);
        assert_eq!(report.readings[0].status, "warming");
        assert_eq!(report.settings.network_selection, "manual");
    }
}
