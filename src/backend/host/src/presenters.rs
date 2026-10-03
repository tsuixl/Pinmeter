use crate::contracts::*;
use pinmeter_core::{application::Monitor, domain::*};

pub fn status(value: Status) -> ReadingStatus {
    match value {
        Status::Normal => ReadingStatus::Normal,
        Status::Warming => ReadingStatus::Warming,
        Status::Unsupported => ReadingStatus::Unsupported,
        Status::PermissionDenied => ReadingStatus::PermissionDenied,
        Status::Failed => ReadingStatus::Failed,
        Status::Stale => ReadingStatus::Stale,
    }
}
pub fn reading(value: &Reading, network: bool) -> ReadingDto {
    let (text, unit) = match value.value {
        Some(number) if value.status == Status::Normal => {
            if network {
                let units = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s", "PB/s"];
                let mut scaled = number;
                let mut index = 0;
                while scaled >= 999.95 && index < units.len() - 1 {
                    scaled /= 1000.;
                    index += 1;
                }
                (
                    if scaled >= 999.95 {
                        "999+".into()
                    } else {
                        format!("{scaled:.1}")
                    },
                    units[index],
                )
            } else {
                (format!("{number:.1}"), "%")
            }
        }
        _ => ("—".into(), if network { "B/s" } else { "%" }),
    };
    ReadingDto {
        value: value.value,
        text,
        unit: unit.into(),
        status: status(value.status),
        valid_at_ms: value.valid_at_ms.map(|t| t as f64),
        source: value.source.into(),
        semantic: value.semantic.into(),
        detail: value.detail.clone(),
    }
}

fn memory_text(value: Option<u64>) -> String {
    value
        .map(|bytes| {
            if bytes >= 1024 * 1024 * 1024 {
                format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            } else {
                format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
            }
        })
        .unwrap_or_else(|| "—".into())
}
fn temperature_reading(value: &Reading) -> ReadingDto {
    let mut dto = reading(value, false);
    dto.unit = "°C".into();
    dto
}
pub fn frame(value: &Frame) -> FrameDto {
    FrameDto {
        gpus: value.gpus.iter().map(gpu_device).collect(),
        cursor: value.cursor.to_string(),
        at_ms: value.at_ms as f64,
        elapsed_ms: value.elapsed_ms as f64,
        generation: value.generation.to_string(),
        network_generation: value.network_generation.to_string(),
        network_id: value.network_id.clone(),
        cpu: reading(&value.cpu, false),
        cpu_temperature: temperature_reading(&value.cpu_temperature),
        memory: reading(&value.memory, false),
        download: reading(&value.download, true),
        upload: reading(&value.upload, true),
        memory_used: memory_text(value.memory_used),
        memory_total: memory_text(value.memory_total),
    }
}
pub fn interface(value: &NetworkInterface) -> InterfaceDto {
    InterfaceDto {
        id: value.id.clone(),
        name: value.name.clone(),
        up: value.up,
        physical: value.physical,
    }
}
pub fn settings(value: &Settings) -> SettingsDto {
    SettingsDto {
        start_in_tray: value.start_in_tray,
        autostart: value.autostart,
        close_action: value.close_action.clone(),
        taskbar: TaskbarSettingsDto {
            enabled: value.taskbar.enabled,
            hidden: value.taskbar.hidden,
            layout: value.taskbar.layout.clone(),
            cpu: value.taskbar.cpu,
            gpu: value.taskbar.gpu,
            memory: value.taskbar.memory,
        },
        revision: value.revision.to_string(),
        theme: value.theme.clone(),
        interval_ms: value.interval_ms as u32,
        release_network_on_exit: value.release_network_on_exit,
        network_id: value.network_id.clone(),
    }
}
pub fn state(monitor: &Monitor, session: &str, after: u64) -> MonitorStateDto {
    use pinmeter_core::ports::Clock;
    let now = pinmeter_platform::shared::SystemClock::default().monotonic_ms();
    let latest_frame = monitor.latest_at(now);
    let processors = monitor.cpu_processors_at(now);
    let temperature = monitor.cpu_temperature_at(now);
    let temperature_dto = temperature_reading(&temperature);
    let latest = latest_frame.as_ref();
    let mut capabilities = latest
        .map(|f| {
            [
                ("cpu", &f.cpu),
                ("memory", &f.memory),
                ("network", &f.download),
            ]
            .map(|(metric, r)| CapabilityDto {
                metric: metric.into(),
                available: matches!(r.status, Status::Normal | Status::Warming),
                status: status(r.status),
                reason: r.detail.clone(),
            })
            .to_vec()
        })
        .unwrap_or_default();
    capabilities.push(CapabilityDto {
        metric: "cpu_temperature".into(),
        available: temperature.status == Status::Normal,
        status: status(temperature.status),
        reason: temperature.detail.clone(),
    });
    MonitorStateDto {
        autostart: AutostartStatusDto {
            available: monitor.autostart.available,
            enabled: monitor.autostart.enabled,
            detail: monitor.autostart.detail.clone(),
        },
        desktop: DesktopStatusDto {
            supported: monitor.desktop.supported,
            stage: monitor.desktop.stage.clone(),
            detail: monitor.desktop.detail.clone(),
            revision: monitor.desktop.revision.to_string(),
        },
        network_control: None,
        ip: None,
        cpu_model: monitor.cpu_model.clone(),
        gpu: gpu_snapshot(&monitor.gpu.at(now)),
        app_network: app_network(&monitor.app_network, now, monitor.settings.interval_ms),
        protocol_version: 1,
        session_id: session.into(),
        platform: pinmeter_platform::platform_name().into(),
        settings: settings(&monitor.settings),
        applied_settings_revision: monitor.applied_revision.to_string(),
        capabilities_version: latest.map_or(0, |f| f.cursor).to_string(),
        capabilities,
        interfaces: monitor.interfaces.iter().map(interface).collect(),
        selected_interface: monitor.selected.as_ref().map(interface),
        frame: latest.map(frame),
        cpu_temperature: temperature_dto,
        cpu_processors: CpuProcessorsDto {
            processors: processors
                .processors
                .iter()
                .map(|p| ProcessorDto {
                    id: p.id.clone(),
                    usage: reading(&p.usage, false),
                })
                .collect(),
            status: status(processors.status),
            detail: processors.detail,
        },
        history: monitor
            .history
            .iter()
            .filter(|f| f.cursor > after)
            .map(frame)
            .collect(),
        diagnostic: monitor.diagnostic.clone(),
    }
}

fn gpu_device(gpu: &pinmeter_core::gpu::GpuDevice) -> GpuDeviceDto {
    use pinmeter_core::gpu::GpuMetric;
    GpuDeviceDto {
        id: gpu.id.clone(),
        name: gpu.name.clone(),
        readings: gpu
            .readings
            .iter()
            .map(|(key, r)| {
                let mut dto = reading(r, false);
                let (scale, unit) = match key {
                    GpuMetric::Usage => (1.0, "%"),
                    GpuMetric::Temperature | GpuMetric::VrSocTemperature => (1.0, "°C"),
                    GpuMetric::CoreClock | GpuMetric::MemoryClock => (1.0, "MHz"),
                    _ => (1073741824.0, "GiB"),
                };
                dto.unit = unit.into();
                if let Some(v) = r.value.filter(|_| r.status == Status::Normal) {
                    dto.text = format!("{:.2}", v / scale);
                }
                (key.key().into(), dto)
            })
            .collect(),
    }
}
fn gpu_snapshot(gpu: &pinmeter_core::gpu::GpuSnapshot) -> GpuSnapshotDto {
    GpuSnapshotDto {
        devices: gpu.devices.iter().map(gpu_device).collect(),
        status: status(gpu.status),
        detail: gpu.detail.clone(),
    }
}

pub fn app_network(
    network: &pinmeter_core::app_network::AppNetwork,
    now: u64,
    interval: u64,
) -> AppNetworkDto {
    let (status, detail) = network.status_at(now, interval);
    let valid = status == "normal";
    let traffic = |t: &pinmeter_core::app_network::Traffic| AppTrafficDto {
        download: valid.then_some(t.download),
        upload: valid.then_some(t.upload),
        received: t.received.to_string(),
        sent: t.sent.to_string(),
        download_share: if valid { t.download_share } else { None },
        upload_share: if valid { t.upload_share } else { None },
    };
    AppNetworkDto {
        session: network.generation.to_string(),
        status: status.into(),
        detail: detail.into(),
        running: network.requested,
        incomplete: network.incomplete,
        limited: network.limited,
        unknown: traffic(&network.unknown),
        other: traffic(&network.other),
        apps: network
            .apps
            .values()
            .map(|app| AppNetworkRowDto {
                icon: app
                    .icon
                    .as_ref()
                    .map(|png| format!("data:image/png;base64,{png}")),
                id: app.id.clone(),
                name: app.name.clone(),
                path: app.path.clone(),
                traffic: traffic(&app.traffic),
                processes: app
                    .processes
                    .values()
                    .map(|p| AppProcessDto {
                        id: p.id.clone(),
                        pid: p.pid,
                        observed: valid && p.observed,
                        traffic: traffic(&p.traffic),
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod taskbar_format_tests {
    use super::*;
    #[test]
    fn shared_speed_format_fits_the_fixed_taskbar_value_slot() {
        for (value, text, unit) in [
            (0., "0.0", "B/s"),
            (999.94, "999.9", "B/s"),
            (999.95, "1.0", "KB/s"),
            (999_950., "1.0", "MB/s"),
            (1e12, "1.0", "TB/s"),
            (1e19, "999+", "PB/s"),
        ] {
            let dto = reading(
                &Reading {
                    value: Some(value),
                    status: Status::Normal,
                    valid_at_ms: Some(1),
                    valid_mono_ms: Some(1),
                    source: "fixture",
                    semantic: "test",
                    detail: String::new(),
                },
                true,
            );
            assert_eq!((dto.text.as_str(), dto.unit.as_str()), (text, unit));
            assert!(dto.text.len() <= 5);
        }
    }
}
