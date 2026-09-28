use crate::domain::{Failure, Observation, Reading, Status};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuMetric {
    Usage,
    DedicatedUsed,
    SharedUsed,
    MemoryTotal,
    Temperature,
    VrSocTemperature,
    CoreClock,
    MemoryClock,
}
impl GpuMetric {
    pub const ALL: [Self; 8] = [
        Self::Usage,
        Self::DedicatedUsed,
        Self::SharedUsed,
        Self::MemoryTotal,
        Self::Temperature,
        Self::VrSocTemperature,
        Self::CoreClock,
        Self::MemoryClock,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::DedicatedUsed => "dedicated_used",
            Self::SharedUsed => "shared_used",
            Self::MemoryTotal => "memory_total",
            Self::Temperature => "temperature",
            Self::VrSocTemperature => "vr_soc_temperature",
            Self::CoreClock => "core_clock",
            Self::MemoryClock => "memory_clock",
        }
    }
    pub fn semantic(self) -> &'static str {
        match self {
            Self::Usage => "gpu.d3d_busiest_engine_percent",
            Self::DedicatedUsed => "gpu.d3d_dedicated_used_bytes",
            Self::SharedUsed => "gpu.d3d_shared_used_bytes",
            Self::MemoryTotal => "gpu.vendor_capacity_bytes",
            Self::Temperature => "gpu.core_celsius",
            Self::VrSocTemperature => "gpu.vr_soc_celsius",
            Self::CoreClock => "gpu.core_mhz",
            Self::MemoryClock => "gpu.memory_driver_mhz",
        }
    }
    pub fn source(self) -> &'static str {
        match self {
            Self::Usage | Self::DedicatedUsed | Self::SharedUsed => {
                "Windows D3D / LibreHardwareMonitor 0.9.6"
            }
            _ => "GPU driver / LibreHardwareMonitor 0.9.6",
        }
    }
    fn valid(self, value: f64) -> bool {
        let (min, max) = match self {
            Self::Usage => (0.0, 100.0),
            Self::Temperature | Self::VrSocTemperature => (-50.0, 150.0),
            Self::CoreClock | Self::MemoryClock => (0.0, 100_000.0),
            Self::MemoryTotal => (1.0, (1_u64 << 50) as f64),
            _ => (0.0, (1_u64 << 50) as f64),
        };
        value.is_finite() && (min..=max).contains(&value)
    }
}
#[derive(Clone, Debug)]
pub struct GpuSample {
    pub id: String,
    pub name: String,
    pub readings: BTreeMap<GpuMetric, Result<f64, Failure>>,
}
#[derive(Clone, Debug)]
pub struct GpuDevice {
    pub id: String,
    pub name: String,
    pub readings: BTreeMap<GpuMetric, Reading>,
}
#[derive(Clone, Debug)]
pub struct GpuSnapshot {
    pub devices: Vec<GpuDevice>,
    pub status: Status,
    pub detail: String,
    sampled_at: Option<u64>,
}
impl Default for GpuSnapshot {
    fn default() -> Self {
        Self {
            devices: vec![],
            status: Status::Warming,
            detail: "等待 GPU 采样".into(),
            sampled_at: None,
        }
    }
}
impl GpuSnapshot {
    pub fn accept(&mut self, sample: Observation<Vec<GpuSample>>) {
        if self.sampled_at.is_some_and(|at| sample.mono_ms <= at) {
            return;
        }
        let mut ids = HashSet::new();
        let result = sample.result.and_then(|rows| {
            if rows.len() > 16
                || rows.iter().any(|r| {
                    r.id.is_empty()
                        || r.id.len() > 512
                        || r.name.len() > 256
                        || !ids.insert(r.id.clone())
                })
            {
                Err(Failure::new(Status::Failed, "GPU 设备列表无效"))
            } else {
                Ok(rows)
            }
        });
        match result {
            Ok(rows) => {
                self.status = if rows.is_empty() {
                    Status::Unsupported
                } else {
                    Status::Normal
                };
                self.detail = if rows.is_empty() {
                    "未发现受支持的物理 GPU".into()
                } else {
                    String::new()
                };
                self.devices = rows
                    .into_iter()
                    .map(|row| {
                        let readings = GpuMetric::ALL
                            .into_iter()
                            .map(|key| {
                                let result = row
                                    .readings
                                    .get(&key)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        Err(Failure::new(Status::Unsupported, "此显卡未提供该指标"))
                                    })
                                    .and_then(|v| {
                                        if key.valid(v) {
                                            Ok(v)
                                        } else {
                                            Err(Failure::new(
                                                Status::Failed,
                                                "GPU 读数超出有效范围",
                                            ))
                                        }
                                    });
                                let (value, status, detail) = match result {
                                    Ok(v) => (Some(v), Status::Normal, String::new()),
                                    Err(e) => (None, e.status, e.reason),
                                };
                                (
                                    key,
                                    Reading {
                                        value,
                                        status,
                                        detail,
                                        source: key.source(),
                                        semantic: key.semantic(),
                                        valid_at_ms: value.map(|_| sample.wall_ms),
                                        valid_mono_ms: value.map(|_| sample.mono_ms),
                                    },
                                )
                            })
                            .collect();
                        GpuDevice {
                            id: row.id,
                            name: row.name,
                            readings,
                        }
                    })
                    .collect();
            }
            Err(error) => {
                self.status = error.status;
                self.detail = error.reason;
                for gpu in &mut self.devices {
                    for reading in gpu.readings.values_mut() {
                        reading.value = None;
                        reading.status = self.status;
                        reading.detail.clone_from(&self.detail);
                    }
                }
            }
        }
        self.sampled_at = Some(sample.mono_ms);
    }
    pub fn at(&self, now: u64) -> Self {
        let mut state = self.clone();
        if matches!(
            state.status,
            Status::Normal | Status::Warming | Status::Failed
        ) && state
            .sampled_at
            .is_some_and(|at| now.saturating_sub(at) > 3000)
        {
            state.status = Status::Stale;
            state.detail = "未在预期间隔内获得 GPU 数据".into();
        }
        for gpu in &mut state.devices {
            for r in gpu.readings.values_mut() {
                if matches!(r.status, Status::Normal | Status::Warming | Status::Failed)
                    && now.saturating_sub(r.valid_mono_ms.or(state.sampled_at).unwrap_or(now))
                        > 3000
                {
                    r.status = Status::Stale;
                    r.value = None;
                    r.detail = "GPU 数据已过期".into();
                }
            }
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(at: u64, value: f64) -> Observation<Vec<GpuSample>> {
        Observation {
            result: Ok(vec![GpuSample {
                id: "card-a".into(),
                name: "GPU".into(),
                readings: [(GpuMetric::Usage, Ok(value))].into(),
            }]),
            source: "test",
            semantic: "test",
            mono_ms: at,
            wall_ms: at + 100,
        }
    }
    #[test]
    fn zero_missing_invalid_and_expiry_are_independent() {
        let mut gpu = GpuSnapshot::default();
        gpu.accept(sample(10, 0.0));
        assert_eq!(gpu.devices[0].readings[&GpuMetric::Usage].value, Some(0.0));
        assert_eq!(
            gpu.devices[0].readings[&GpuMetric::Temperature].status,
            Status::Unsupported
        );
        let frozen = gpu.at(10);
        gpu.accept(sample(10, 80.0));
        gpu.accept(sample(9, 80.0));
        assert_eq!(
            gpu.at(3011).devices[0].readings[&GpuMetric::Usage].status,
            Status::Stale
        );
        assert_eq!(
            frozen.devices[0].readings[&GpuMetric::Usage].value,
            Some(0.0)
        );
        gpu.accept(sample(4000, f64::NAN));
        assert_eq!(
            gpu.devices[0].readings[&GpuMetric::Usage].status,
            Status::Failed
        );
    }
    #[test]
    fn vr_soc_temperature_has_independent_identity_validity_and_history() {
        let mut gpu = GpuSnapshot::default();
        let mut s = sample(10, 0.0);
        s.result.as_mut().unwrap()[0]
            .readings
            .insert(GpuMetric::VrSocTemperature, Ok(56.0));
        gpu.accept(s);
        let frozen = gpu.at(10);
        let readings = &frozen.devices[0].readings;
        assert_eq!(
            readings[&GpuMetric::Temperature].status,
            Status::Unsupported
        );
        assert_eq!(readings[&GpuMetric::VrSocTemperature].value, Some(56.0));
        assert_eq!(
            readings[&GpuMetric::VrSocTemperature].semantic,
            "gpu.vr_soc_celsius"
        );
        assert_eq!(
            gpu.at(3011).devices[0].readings[&GpuMetric::VrSocTemperature].status,
            Status::Stale
        );
        assert_eq!(readings[&GpuMetric::VrSocTemperature].value, Some(56.0));
    }
    #[test]
    fn device_removal_and_duplicate_identity_do_not_merge_cards() {
        let mut gpu = GpuSnapshot::default();
        let mut s = sample(10, 1.0);
        let row = s.result.as_ref().unwrap()[0].clone();
        s.result.as_mut().unwrap().push(row);
        gpu.accept(s);
        assert_eq!(gpu.status, Status::Failed);
        gpu.accept(sample(20, 1.0));
        let mut removed = sample(30, 1.0);
        removed.result = Ok(vec![]);
        gpu.accept(removed);
        assert!(gpu.devices.is_empty());
        assert_eq!(gpu.status, Status::Unsupported);
    }
}
