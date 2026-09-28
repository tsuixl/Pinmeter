use crate::{domain::*, ports::SettingsRepository};
use std::collections::VecDeque;

/// A validated change owns only settings, never a copy of live monitoring state.
/// The runtime serializes writers until its outcome has been accepted.
pub struct SettingsChange {
    next: Settings,
    change_autostart: bool,
}
pub struct SettingsOutcome {
    result: Result<Settings, String>,
    autostart: Option<crate::autostart::AutostartStatus>,
}
impl SettingsChange {
    pub fn execute(
        self,
        repository: &dyn SettingsRepository,
        autostart: &dyn crate::autostart::Autostart,
    ) -> SettingsOutcome {
        let mut actual = None;
        let result = (|| {
            let previous = if self.change_autostart {
                let checkpoint = autostart.checkpoint()?;
                if let Err(error) = autostart.set_enabled(self.next.autostart) {
                    let rollback = autostart.restore(&checkpoint);
                    actual = Some(autostart.status());
                    return Err(match rollback {
                        Ok(()) => error,
                        Err(rollback) => {
                            format!("{error}；自启恢复失败：{rollback}，请检查系统启动项")
                        }
                    });
                }
                Some(checkpoint)
            } else {
                None
            };
            if let Err(error) = repository.save(&self.next) {
                if let Some(previous) = previous {
                    let rollback = autostart.restore(&previous);
                    actual = Some(autostart.status());
                    if let Err(rollback) = rollback {
                        return Err(format!("{error}；自启恢复失败：{rollback}，请重新操作开关"));
                    }
                }
                return Err(error);
            }
            if self.change_autostart {
                actual = Some(autostart.status());
            }
            Ok(self.next)
        })();
        SettingsOutcome {
            result,
            autostart: actual,
        }
    }
}

pub struct Monitor {
    pub autostart: crate::autostart::AutostartStatus,
    pub desktop: crate::desktop::DesktopStatus,
    pub cpu_model: Option<String>,
    pub gpu: crate::gpu::GpuSnapshot,
    pub app_network: crate::app_network::AppNetwork,
    pub settings: Settings,
    pub applied_revision: u64,
    pub history: VecDeque<Frame>,
    pub interfaces: Vec<NetworkInterface>,
    pub selected: Option<NetworkInterface>,
    pub diagnostic: Option<String>,
    cpu_processors: CpuProcessors,
    cpu_temperature: Reading,
    temperature_sampled_at: u64,
    baseline: RateBaseline,
    generation: u64,
    cursor: u64,
}
impl Monitor {
    pub fn new(settings: Settings, diagnostic: Option<String>) -> Self {
        Self {
            autostart: Default::default(),
            desktop: Default::default(),
            cpu_model: None,
            gpu: crate::gpu::GpuSnapshot::default(),
            app_network: crate::app_network::AppNetwork::default(),
            applied_revision: 0,
            settings,
            history: VecDeque::with_capacity(301),
            interfaces: vec![],
            selected: None,
            diagnostic,
            cpu_temperature: Reading {
                value: None,
                status: Status::Warming,
                valid_at_ms: None,
                valid_mono_ms: None,
                source: "CPU temperature sensor",
                semantic: "cpu.package_die_or_core_max_celsius",
                detail: "等待 CPU 温度采样".into(),
            },
            temperature_sampled_at: 0,
            cpu_processors: CpuProcessors {
                processors: vec![],
                status: Status::Warming,
                detail: "等待逻辑处理器采样".into(),
                sampled_mono_ms: 0,
            },
            baseline: RateBaseline::default(),
            generation: 0,
            cursor: 0,
        }
    }
    pub fn update_settings(
        &mut self,
        next: Settings,
        expected: u64,
        repository: &dyn SettingsRepository,
        autostart: &dyn crate::autostart::Autostart,
    ) -> Result<(), String> {
        let change = self.prepare_settings(next, expected)?;
        self.finish_settings(change.execute(repository, autostart))
    }
    pub fn prepare_settings(
        &self,
        mut next: Settings,
        expected: u64,
    ) -> Result<SettingsChange, String> {
        next.validate()?;
        if expected != self.settings.revision {
            return Err(format!(
                "配置版本冲突，当前版本为 {}；请放弃更改后重新编辑",
                self.settings.revision
            ));
        }
        next.revision = self
            .settings
            .revision
            .checked_add(1)
            .ok_or("配置版本已耗尽")?;
        Ok(SettingsChange {
            change_autostart: next.autostart != self.settings.autostart,
            next,
        })
    }
    pub fn finish_settings(&mut self, outcome: SettingsOutcome) -> Result<(), String> {
        if let Some(actual) = outcome.autostart {
            self.autostart = actual;
        }
        let next = outcome.result?;
        if self.settings.network_id != next.network_id {
            self.baseline.reset();
            self.selected = None;
        }
        if self.settings.interval_ms != next.interval_ms {
            self.baseline.reset();
            self.generation += 1;
        }
        self.settings = next;
        self.diagnostic = None;
        Ok(())
    }
    pub fn reset_baseline(&mut self) {
        self.baseline.reset();
        self.generation += 1;
    }
    pub fn accept_temperature(&mut self, sample: Observation<f64>) {
        if sample.mono_ms < self.temperature_sampled_at {
            return;
        }
        let result = sample.result.clone().and_then(|value| {
            if value.is_finite() && (-50.0..=150.0).contains(&value) {
                Ok(value)
            } else {
                Err(Failure::new(Status::Failed, "CPU 温度超出有效范围"))
            }
        });
        self.cpu_temperature = reading(result, &sample, Some(&self.cpu_temperature), 3000);
        self.temperature_sampled_at = sample.mono_ms;
    }
    pub fn cpu_temperature_at(&self, now: u64) -> Reading {
        let mut value = self.cpu_temperature.clone();
        let (sampled_at, max_age) = if value.status == Status::Warming {
            (self.temperature_sampled_at, 120_000)
        } else {
            (
                value.valid_mono_ms.unwrap_or(self.temperature_sampled_at),
                3000,
            )
        };
        if matches!(
            value.status,
            Status::Normal | Status::Warming | Status::Failed
        ) && now.saturating_sub(sampled_at) > max_age
        {
            value.status = Status::Stale;
            value.value = None;
            value.detail = "未在预期间隔内获得有效 CPU 温度".into();
        }
        value
    }
    /// Current validity is evaluated on read even if the producer has stopped.
    /// Historical frames retain the status they had at collection time.
    pub fn latest_at(&self, now_ms: u64) -> Option<Frame> {
        let mut frame = self.history.back()?.clone();
        frame.cpu_temperature = self.cpu_temperature_at(now_ms);
        frame.gpus = self.gpu.at(now_ms).devices;
        let max_age = self.settings.interval_ms * 3;
        for reading in [
            &mut frame.cpu,
            &mut frame.memory,
            &mut frame.download,
            &mut frame.upload,
        ] {
            let at = if reading.status == Status::Warming {
                frame.elapsed_ms
            } else {
                reading.valid_mono_ms.unwrap_or(frame.elapsed_ms)
            };
            if matches!(
                reading.status,
                Status::Normal | Status::Warming | Status::Failed
            ) && now_ms.saturating_sub(at) > max_age
            {
                reading.status = Status::Stale;
                reading.value = None;
                reading.detail = "未在预期间隔内获得有效采样".into();
            }
        }
        if frame.memory.status != Status::Normal {
            frame.memory_used = None;
            frame.memory_total = None;
        }
        Some(frame)
    }
    pub fn accept(&mut self, sample: RawSample, revision: u64) -> bool {
        if revision != self.settings.revision {
            return false;
        }
        let now = sample
            .network
            .mono_ms
            .max(sample.cpu.mono_ms)
            .max(sample.memory.mono_ms);
        if self.history.back().is_some_and(|f| now <= f.elapsed_ms) {
            return false;
        }
        self.accept_processors(&sample.cpu_processors);
        let max_gap = self.settings.interval_ms * 3;
        if self
            .history
            .back()
            .is_some_and(|f| now - f.elapsed_ms > max_gap)
        {
            self.reset_baseline();
        }
        let prev = self.history.back();
        let cpu_result = sample.cpu.result.clone().and_then(|v| {
            if v.is_finite() && (0.0..=100.0).contains(&v) {
                Ok(v)
            } else {
                Err(Failure::new(Status::Failed, "CPU 读数超出有效范围"))
            }
        });
        let memory_result = sample.memory.result.clone().and_then(|m| {
            if m.total > 0 && m.used <= m.total {
                Ok(m)
            } else {
                Err(Failure::new(Status::Failed, "无效的物理内存读数"))
            }
        });
        let cpu = reading(cpu_result, &sample.cpu, prev.map(|f| &f.cpu), max_gap);
        let memory = reading(
            memory_result
                .clone()
                .map(|m| m.used as f64 / m.total as f64 * 100.0),
            &sample.memory,
            prev.map(|f| &f.memory),
            max_gap,
        );
        let selected = match &sample.network.result {
            Ok(interfaces) => {
                self.interfaces = interfaces.iter().map(|n| n.interface.clone()).collect();
                self.interfaces.sort_by(|a, b| a.id.cmp(&b.id));
                if let Some(id) = &self.settings.network_id {
                    interfaces
                        .iter()
                        .find(|n| &n.interface.id == id && n.interface.up)
                } else {
                    self.selected
                        .as_ref()
                        .and_then(|old| {
                            interfaces
                                .iter()
                                .find(|n| n.interface.id == old.id && n.interface.up)
                        })
                        .or_else(|| {
                            interfaces
                                .iter()
                                .filter(|n| n.interface.up)
                                .min_by_key(|n| (!n.interface.physical, n.interface.id.clone()))
                        })
                }
            }
            Err(_) => None,
        };
        self.selected = selected.map(|n| n.interface.clone());
        let rates = if let Some(counters) = selected {
            self.baseline
                .rates(counters, sample.network.mono_ms, max_gap)
        } else {
            self.baseline.reset();
            Err(sample
                .network
                .result
                .as_ref()
                .err()
                .cloned()
                .unwrap_or_else(|| Failure::new(Status::Failed, "所选网卡未连接或已移除")))
        };
        let prev = self.history.back();
        let download = reading(
            rates.clone().map(|(d, _)| d),
            &sample.network,
            prev.map(|f| &f.download),
            max_gap,
        );
        let upload = reading(
            rates.map(|(_, u)| u),
            &sample.network,
            prev.map(|f| &f.upload),
            max_gap,
        );
        self.cursor += 1;
        let frame = Frame {
            gpus: self.gpu.at(now).devices,
            cursor: self.cursor,
            at_ms: sample
                .network
                .wall_ms
                .max(sample.cpu.wall_ms)
                .max(sample.memory.wall_ms),
            elapsed_ms: now,
            generation: self.generation,
            network_generation: self.baseline.generation,
            network_id: self
                .selected
                .as_ref()
                .map(|n| n.id.clone())
                .or_else(|| self.settings.network_id.clone()),
            cpu,
            cpu_temperature: self.cpu_temperature_at(now),
            memory,
            download,
            upload,
            memory_used: memory_result.as_ref().ok().map(|m| m.used),
            memory_total: memory_result.ok().map(|m| m.total),
        };
        self.history.push_back(frame);
        self.applied_revision = revision;
        while self.history.len() > 301
            || self
                .history
                .front()
                .is_some_and(|f| now.saturating_sub(f.elapsed_ms) > 300_000)
        {
            self.history.pop_front();
        }
        true
    }

    pub fn cpu_processors_at(&self, now_ms: u64) -> CpuProcessors {
        let mut snapshot = self.cpu_processors.clone();
        let max_age = self.settings.interval_ms * 3;
        if matches!(
            snapshot.status,
            Status::Normal | Status::Warming | Status::Failed
        ) && now_ms.saturating_sub(snapshot.sampled_mono_ms) > max_age
        {
            snapshot.status = Status::Stale;
            snapshot.detail = "未在预期间隔内获得逻辑处理器采样".into();
        }
        for processor in &mut snapshot.processors {
            let usage = &mut processor.usage;
            if matches!(
                usage.status,
                Status::Normal | Status::Warming | Status::Failed
            ) && now_ms.saturating_sub(usage.valid_mono_ms.unwrap_or(snapshot.sampled_mono_ms))
                > max_age
            {
                usage.status = Status::Stale;
                usage.value = None;
                usage.detail = "未在预期间隔内获得有效采样".into();
            }
        }
        snapshot
    }

    fn accept_processors(&mut self, observation: &Observation<Vec<ProcessorSample>>) {
        let mut ids = std::collections::HashSet::new();
        let result = observation
            .result
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|rows| {
                if rows.is_empty()
                    || rows.len() > 4096
                    || rows.iter().any(|p| p.id.is_empty() || !ids.insert(&p.id))
                {
                    Err(Failure::new(
                        Status::Failed,
                        "逻辑处理器列表为空、过大或编号重复",
                    ))
                } else {
                    Ok(rows)
                }
            });
        let previous = &self.cpu_processors.processors;
        let max_age = self.settings.interval_ms * 3;
        let (processors, status, detail) = match result {
            Ok(rows) => (
                rows.iter()
                    .map(|p| ProcessorReading {
                        id: p.id.clone(),
                        usage: reading(
                            p.usage.clone().and_then(|value| {
                                if value.is_finite() && (0.0..=100.0).contains(&value) {
                                    Ok(value)
                                } else {
                                    Err(Failure::new(Status::Failed, "逻辑处理器占用超出有效范围"))
                                }
                            }),
                            observation,
                            previous
                                .iter()
                                .find(|old| old.id == p.id)
                                .map(|old| &old.usage),
                            max_age,
                        ),
                    })
                    .collect(),
                Status::Normal,
                String::new(),
            ),
            Err(error) => (
                previous
                    .iter()
                    .map(|p| ProcessorReading {
                        id: p.id.clone(),
                        usage: reading(Err(error.clone()), observation, Some(&p.usage), max_age),
                    })
                    .collect(),
                error.status,
                error.reason,
            ),
        };
        self.cpu_processors = CpuProcessors {
            processors,
            status,
            detail,
            sampled_mono_ms: observation.mono_ms,
        };
    }
}

fn reading<T>(
    result: Result<f64, Failure>,
    observation: &Observation<T>,
    previous: Option<&Reading>,
    max_age: u64,
) -> Reading {
    match result {
        Ok(value) => Reading {
            value: Some(value),
            status: Status::Normal,
            valid_at_ms: Some(observation.wall_ms),
            valid_mono_ms: Some(observation.mono_ms),
            source: observation.source,
            semantic: observation.semantic,
            detail: String::new(),
        },
        Err(error) => {
            let last = previous.and_then(|p| p.valid_mono_ms);
            let stale = last.is_some_and(|at| observation.mono_ms.saturating_sub(at) > max_age)
                && error.status == Status::Failed;
            Reading {
                value: None,
                status: if stale { Status::Stale } else { error.status },
                valid_at_ms: previous.and_then(|p| p.valid_at_ms),
                valid_mono_ms: last,
                source: observation.source,
                semantic: observation.semantic,
                detail: error.reason,
            }
        }
    }
}
