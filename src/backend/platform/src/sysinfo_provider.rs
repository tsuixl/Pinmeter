use crate::shared::SystemClock;
use pinmeter_core::{domain::*, ports::MetricProvider};
use sysinfo::{Networks, System};

pub fn cpu_model() -> Option<String> {
    let mut system = System::new();
    system.refresh_cpu_all();
    system.cpus().first().map(|cpu| cpu.brand().to_owned())
}

pub struct SysinfoProvider {
    system: System,
    clock: SystemClock,
    primed: bool,
}
impl SysinfoProvider {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            clock: SystemClock::default(),
            primed: false,
        }
    }
}
impl MetricProvider for SysinfoProvider {
    fn sample(&mut self) -> RawSample {
        self.system.refresh_cpu_usage();
        let cpu_result = if !sysinfo::IS_SUPPORTED_SYSTEM || self.system.cpus().is_empty() {
            Err(Failure::new(Status::Unsupported, "系统未提供 CPU 计数"))
        } else if !self.primed {
            self.primed = true;
            Err(Failure::new(Status::Warming, "CPU 正在预热"))
        } else {
            Ok(self.system.global_cpu_usage() as f64)
        };
        let cpu_processors = self.clock.observation(
            if self.system.cpus().is_empty() {
                Err(Failure::new(
                    Status::Unsupported,
                    "系统未提供逻辑处理器列表",
                ))
            } else {
                Ok(self
                    .system
                    .cpus()
                    .iter()
                    .enumerate()
                    .map(|(index, processor)| ProcessorSample {
                        id: index.to_string(),
                        usage: cpu_result
                            .as_ref()
                            .map(|_| processor.cpu_usage() as f64)
                            .map_err(Clone::clone),
                    })
                    .collect())
            },
            "sysinfo 0.39.6 CPU",
            "cpu.logical_processor.busy_time",
        );
        let cpu = self
            .clock
            .observation(cpu_result, "sysinfo 0.39.6 CPU", "cpu.busy_time");
        // Fresh memory/network objects prevent a failed refresh from re-publishing cached gauges.
        let mut memory_system = System::new();
        memory_system.refresh_memory();
        let memory_result = if memory_system.total_memory() == 0 {
            Err(Failure::new(Status::Failed, "系统未返回物理内存"))
        } else {
            Ok(Memory {
                used: memory_system.used_memory(),
                total: memory_system.total_memory(),
            })
        };
        let memory = self.clock.observation(
            memory_result,
            "sysinfo 0.39.6 memory",
            "memory.physical.platform_used",
        );
        let networks = Networks::new_with_refreshed_list();
        let rows: Vec<_> = networks
            .iter()
            .filter(|(name, _)| name.as_str() != "lo" && name.as_str() != "lo0")
            .map(|(name, data)| NetworkCounters {
                interface: NetworkInterface {
                    id: format!("net:{name}:{}", data.mac_address()),
                    name: name.clone(),
                    up: !data.ip_networks().is_empty(),
                    physical: false,
                },
                received: data.total_received(),
                transmitted: data.total_transmitted(),
            })
            .collect();
        let network_result = if rows.is_empty() {
            Err(Failure::new(Status::Failed, "没有可读取的网络接口"))
        } else {
            Ok(rows)
        };
        let network = self.clock.observation(
            network_result,
            "sysinfo 0.39.6 network",
            "network.bytes_per_second",
        );
        RawSample {
            cpu,
            cpu_processors,
            memory,
            network,
        }
    }
    fn reset_baseline(&mut self) {
        self.primed = false;
        self.system = System::new();
    }
}
