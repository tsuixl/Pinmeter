use crate::shared::SystemClock;
use pinmeter_core::{domain::*, ports::MetricProvider};
use windows::{
    Win32::{
        NetworkManagement::{IpHelper::*, Ndis::IfOperStatusUp},
        System::{
            Performance::*,
            SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        },
    },
    core::{PCWSTR, w},
};
// pdh.h: this flag is omitted from the windows-rs metadata projection.
const PDH_FMT_NOCAP100: PDH_FMT = PDH_FMT(0x8000);

pub fn cpu_model() -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let mut buffer = [0u16; 512];
    let mut bytes = std::mem::size_of_val(&buffer) as u32;
    // SAFETY: the UTF-16 output buffer is writable for the supplied byte count;
    // RegGetValueW opens/closes the subkey and restricts the value to REG_SZ.
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
            w!("ProcessorNameString"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .ok()
        .ok()?;
    }
    let end = buffer.iter().position(|unit| *unit == 0)?;
    String::from_utf16(&buffer[..end]).ok()
}

struct CpuQuery {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    processor_counter: Option<PDH_HCOUNTER>,
    processor_result: Result<Vec<ProcessorSample>, Failure>,
    primed: bool,
}
impl CpuQuery {
    fn new() -> Result<Self, Failure> {
        let mut query = PDH_HQUERY::default();
        // SAFETY: output handles are initialized by PDH and owned only by this worker.
        check(
            unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) },
            "PdhOpenQuery",
        )?;
        let mut cpu = Self {
            query,
            counter: PDH_HCOUNTER::default(),
            processor_counter: None,
            processor_result: Err(Failure::new(Status::Warming, "逻辑处理器正在预热")),
            primed: false,
        };
        check(
            unsafe {
                PdhAddEnglishCounterW(
                    query,
                    w!("\\Processor Information(_Total)\\% Processor Time"),
                    0,
                    &mut cpu.counter,
                )
            },
            "PdhAddEnglishCounter",
        )?;
        Ok(cpu)
    }
    fn sample(&mut self) -> Result<f64, Failure> {
        let added = if self.processor_counter.is_none() {
            let mut counter = PDH_HCOUNTER::default();
            match check(
                unsafe {
                    PdhAddEnglishCounterW(
                        self.query,
                        w!("\\Processor Information(*)\\% Processor Time"),
                        0,
                        &mut counter,
                    )
                },
                "PdhAddEnglishCounter / logical processors",
            ) {
                Ok(()) => {
                    self.processor_counter = Some(counter);
                    true
                }
                Err(error) => {
                    self.processor_result = Err(error);
                    false
                }
            }
        } else {
            false
        };
        let collected = check(
            unsafe { PdhCollectQueryData(self.query) },
            "PdhCollectQueryData",
        );
        if let Err(error) = &collected {
            self.processor_result = Err(error.clone());
        }
        collected?;
        if let Some(counter) = self.processor_counter {
            self.processor_result = processor_values(counter, !self.primed || added);
        }
        if !self.primed {
            self.primed = true;
            return Err(Failure::new(Status::Warming, "CPU 需要两次有效采样"));
        }
        let mut value = PDH_FMT_COUNTERVALUE::default();
        let result = unsafe {
            PdhGetFormattedCounterValue(
                self.counter,
                PDH_FMT(PDH_FMT_DOUBLE.0 | PDH_FMT_NOCAP100.0),
                None,
                &mut value,
            )
        };
        check(result, "PdhGetFormattedCounterValue")?;
        validate_counter(value.CStatus, unsafe { value.Anonymous.doubleValue })
    }
}

fn processor_id(name: &str) -> Option<(u16, u16)> {
    let (group, index) = name.split_once(',')?;
    Some((group.parse().ok()?, index.parse().ok()?))
}

fn processor_values(counter: PDH_HCOUNTER, warming: bool) -> Result<Vec<ProcessorSample>, Failure> {
    if warming {
        return Err(Failure::new(Status::Warming, "CPU 需要两次有效采样"));
    }
    let format = PDH_FMT(PDH_FMT_DOUBLE.0 | PDH_FMT_NOCAP100.0);
    // Query size afresh on each attempt: a topology change can invalidate the previous size.
    for _ in 0..2 {
        let mut bytes = 0;
        let mut count = 0;
        let code =
            unsafe { PdhGetFormattedCounterArrayW(counter, format, &mut bytes, &mut count, None) };
        if code != PDH_MORE_DATA {
            check(code, "PdhGetFormattedCounterArray / size")?;
        }
        if bytes == 0 || bytes > 4 * 1024 * 1024 {
            return Err(Failure::new(Status::Failed, "逻辑处理器缓冲区大小无效"));
        }
        // Typed allocation supplies native struct alignment plus space for UTF-16 instance names.
        let mut buffer = vec![
            PDH_FMT_COUNTERVALUE_ITEM_W::default();
            (bytes as usize).div_ceil(size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>())
        ];
        let code = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                format,
                &mut bytes,
                &mut count,
                Some(buffer.as_mut_ptr()),
            )
        };
        if code == PDH_MORE_DATA {
            continue;
        }
        check(code, "PdhGetFormattedCounterArray")?;
        if count as usize > buffer.len() {
            return Err(Failure::new(Status::Failed, "逻辑处理器数量超出缓冲区"));
        }
        let mut processors = Vec::new();
        for item in &buffer[..count as usize] {
            // SAFETY: PDH writes null-terminated names into the still-live buffer.
            let name = unsafe { item.szName.to_string() }
                .map_err(|_| Failure::new(Status::Failed, "无效的逻辑处理器编号"))?;
            if let Some(id) = processor_id(&name) {
                let usage = validate_counter(item.FmtValue.CStatus, unsafe {
                    item.FmtValue.Anonymous.doubleValue
                });
                processors.push((
                    id,
                    ProcessorSample {
                        id: format!("{},{}", id.0, id.1),
                        usage,
                    },
                ));
            }
        }
        processors.sort_by_key(|(id, _)| *id);
        if processors.is_empty() {
            return Err(Failure::new(Status::Failed, "系统未返回逻辑处理器实例"));
        }
        return Ok(processors
            .into_iter()
            .map(|(_, processor)| processor)
            .collect());
    }
    Err(Failure::new(
        Status::Failed,
        "逻辑处理器列表发生变化，等待下次采样",
    ))
}
impl Drop for CpuQuery {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}
fn check(code: u32, operation: &str) -> Result<(), Failure> {
    if code == 0 {
        Ok(())
    } else {
        Err(Failure::new(
            if code == 5 || code == PDH_ACCESS_DENIED {
                Status::PermissionDenied
            } else {
                Status::Failed
            },
            format!("{operation}: 0x{code:08X}"),
        ))
    }
}
fn validate_counter(status: u32, value: f64) -> Result<f64, Failure> {
    if !matches!(status, PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA) {
        return Err(Failure::new(
            Status::Failed,
            format!("PDH counter: 0x{status:08X}"),
        ));
    }
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(Failure::new(Status::Failed, "PDH CPU 超出有效范围"));
    }
    Ok(value)
}
fn memory() -> Result<Memory, Failure> {
    let mut value = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut value) }.map_err(|error| {
        Failure::new(
            if error.code().0 as u32 == 0x80070005 {
                Status::PermissionDenied
            } else {
                Status::Failed
            },
            format!("GlobalMemoryStatusEx: {error}"),
        )
    })?;
    let used = value
        .ullTotalPhys
        .checked_sub(value.ullAvailPhys)
        .ok_or_else(|| Failure::new(Status::Failed, "可用物理内存超过总量"))?;
    Ok(Memory {
        used,
        total: value.ullTotalPhys,
    })
}
fn network() -> Result<Vec<NetworkCounters>, Failure> {
    let mut pointer = std::ptr::null_mut();
    check(unsafe { GetIfTable2(&mut pointer) }.0, "GetIfTable2")?;
    if pointer.is_null() {
        return Err(Failure::new(Status::Failed, "GetIfTable2 returned null"));
    }
    struct Table(*mut MIB_IF_TABLE2);
    impl Drop for Table {
        fn drop(&mut self) {
            unsafe {
                FreeMibTable(self.0.cast());
            }
        }
    }
    let table = Table(pointer);
    // SAFETY: GetIfTable2 allocates NumEntries rows with native MIB_IF_ROW2 alignment.
    let rows = unsafe {
        std::slice::from_raw_parts((*table.0).Table.as_ptr(), (*table.0).NumEntries as usize)
    };
    Ok(rows
        .iter()
        .filter(|row| row.Type != 24)
        .map(|row| {
            let len = row
                .Alias
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(row.Alias.len());
            NetworkCounters {
                interface: NetworkInterface {
                    id: format!("win:{:?}", row.InterfaceGuid),
                    name: String::from_utf16_lossy(&row.Alias[..len]),
                    up: row.OperStatus == IfOperStatusUp,
                    physical: row.InterfaceAndOperStatusFlags._bitfield & 1 != 0,
                },
                received: row.InOctets,
                transmitted: row.OutOctets,
            }
        })
        .collect())
}
pub struct WindowsProvider {
    clock: SystemClock,
    cpu: Option<CpuQuery>,
    retry: u32,
}
impl WindowsProvider {
    pub fn new() -> Self {
        Self {
            clock: SystemClock::default(),
            cpu: None,
            retry: 0,
        }
    }
}
impl MetricProvider for WindowsProvider {
    fn sample(&mut self) -> RawSample {
        let cpu_result = if self.retry > 0 {
            self.retry -= 1;
            Err(Failure::new(Status::Failed, "CPU 计数器暂不可用，稍后重试"))
        } else {
            if self.cpu.is_none() {
                match CpuQuery::new() {
                    Ok(query) => self.cpu = Some(query),
                    Err(error) => {
                        self.retry = 4;
                        return self.with_cpu(Err(error.clone()), Err(error));
                    }
                }
            }
            let result = self.cpu.as_mut().unwrap().sample();
            let processors = self.cpu.as_ref().unwrap().processor_result.clone();
            if result
                .as_ref()
                .is_err_and(|error| error.status != Status::Warming)
            {
                self.cpu = None;
                self.retry = 4;
            }
            return self.with_cpu(result, processors);
        };
        self.with_cpu(cpu_result.clone(), cpu_result.map(|_| vec![]))
    }
    fn reset_baseline(&mut self) {
        self.cpu = None;
        self.retry = 0;
    }
}
impl WindowsProvider {
    fn with_cpu(
        &self,
        cpu: Result<f64, Failure>,
        processors: Result<Vec<ProcessorSample>, Failure>,
    ) -> RawSample {
        let cpu_processors = self.clock.observation(
            processors,
            "Windows PDH / Processor Information(*) / % Processor Time",
            "cpu.logical_processor.busy_time",
        );
        let cpu = self.clock.observation(
            cpu,
            "Windows PDH / Processor Information(_Total) / % Processor Time",
            "cpu.busy_time",
        );
        let memory = self.clock.observation(
            memory(),
            "Windows GlobalMemoryStatusEx",
            "memory.physical.total_minus_available",
        );
        let network = self.clock.observation(
            network(),
            "Windows GetIfTable2 / InOctets / OutOctets",
            "network.bytes_per_second",
        );
        RawSample {
            cpu,
            cpu_processors,
            memory,
            network,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn processor_instances_exclude_aggregate_counters() {
        assert_eq!(processor_id("_Total"), None);
        assert_eq!(processor_id("0,_Total"), None);
        assert_eq!(processor_id("1,12"), Some((1, 12)));
        assert_eq!(processor_id("0,0"), Some((0, 0)));
        assert_eq!(processor_id("0,1,2"), None);
    }
    #[test]
    fn invalid_counter_is_not_a_zero_or_capped_load() {
        assert!(validate_counter(PDH_CSTATUS_NO_INSTANCE, 0.0).is_err());
        assert!(validate_counter(PDH_CSTATUS_VALID_DATA, 101.0).is_err());
        assert!(validate_counter(PDH_CSTATUS_VALID_DATA, f64::NAN).is_err());
        assert_eq!(validate_counter(PDH_CSTATUS_NEW_DATA, 0.0).unwrap(), 0.0);
    }
}
