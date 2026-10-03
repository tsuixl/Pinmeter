use pinmeter_core::{
    disk::DiskSample,
    domain::{Failure, Status},
};

#[cfg(target_os = "windows")]
pub use native::DiskCollector;
#[cfg(not(target_os = "windows"))]
pub struct DiskCollector;
#[cfg(not(target_os = "windows"))]
impl DiskCollector {
    pub fn new() -> Result<Self, Failure> {
        Err(Failure::new(
            Status::Unsupported,
            "此平台尚未支持磁盘实时采样",
        ))
    }
    pub fn sample(&mut self) -> Result<Vec<DiskSample>, Failure> {
        Err(Failure::new(
            Status::Unsupported,
            "此平台尚未支持磁盘实时采样",
        ))
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use windows::{
        Win32::System::Performance::*,
        core::{PCWSTR, w},
    };
    pub struct DiskCollector {
        query: PDH_HQUERY,
        counters: [PDH_HCOUNTER; 3],
        primed: bool,
    }
    impl DiskCollector {
        pub fn new() -> Result<Self, Failure> {
            let mut query = PDH_HQUERY::default();
            check(unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) })?;
            let mut this = Self {
                query,
                counters: [PDH_HCOUNTER::default(); 3],
                primed: false,
            };
            for (index, path) in [
                w!("\\PhysicalDisk(*)\\Disk Read Bytes/sec"),
                w!("\\PhysicalDisk(*)\\Disk Write Bytes/sec"),
                w!("\\PhysicalDisk(*)\\% Idle Time"),
            ]
            .into_iter()
            .enumerate()
            {
                check(unsafe { PdhAddEnglishCounterW(query, path, 0, &mut this.counters[index]) })?;
            }
            Ok(this)
        }
        pub fn sample(&mut self) -> Result<Vec<DiskSample>, Failure> {
            check(unsafe { PdhCollectQueryData(self.query) })?;
            if !self.primed {
                self.primed = true;
                return Err(Failure::new(Status::Warming, "磁盘需要两次有效采样"));
            }
            let [read, write, idle] = self.counters.map(values);
            let names: BTreeSet<_> = [&read, &write, &idle]
                .iter()
                .filter_map(|v| v.as_ref().ok())
                .flat_map(|v| v.keys().cloned())
                .collect();
            if names.is_empty() {
                for value in [&read, &write, &idle] {
                    if let Err(error) = value {
                        return Err(error.clone());
                    }
                }
            }
            let get = |map: &Result<BTreeMap<String, Result<f64, Failure>>, Failure>,
                       name: &str| {
                map.as_ref().map_err(Clone::clone).and_then(|v| {
                    v.get(name)
                        .cloned()
                        .unwrap_or_else(|| Err(Failure::new(Status::Warming, "磁盘实例正在更新")))
                })
            };
            Ok(names
                .into_iter()
                .take(pinmeter_core::disk::MAX_DISKS + 1)
                .map(|name| DiskSample {
                    read: get(&read, &name),
                    write: get(&write, &name),
                    idle: get(&idle, &name),
                    id: name,
                })
                .collect())
        }
    }
    impl Drop for DiskCollector {
        fn drop(&mut self) {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
    fn check(code: u32) -> Result<(), Failure> {
        if code == 0 {
            Ok(())
        } else {
            Err(Failure::new(
                if code == 5 || code == PDH_ACCESS_DENIED {
                    Status::PermissionDenied
                } else {
                    Status::Failed
                },
                format!("磁盘 PDH 暂不可用（{code:#x}），稍后自动重试"),
            ))
        }
    }
    fn values(counter: PDH_HCOUNTER) -> Result<BTreeMap<String, Result<f64, Failure>>, Failure> {
        let format = PDH_FMT(PDH_FMT_DOUBLE.0 | 0x8000);
        for _ in 0..2 {
            let (mut bytes, mut count) = (0, 0);
            let code = unsafe {
                PdhGetFormattedCounterArrayW(counter, format, &mut bytes, &mut count, None)
            };
            if code != PDH_MORE_DATA {
                check(code)?;
            }
            if bytes == 0 {
                return Ok(BTreeMap::new());
            }
            if bytes > 1024 * 1024 {
                return Err(Failure::new(Status::Failed, "磁盘计数器缓冲区超限"));
            }
            let mut buffer = vec![
                PDH_FMT_COUNTERVALUE_ITEM_W::default();
                (bytes as usize)
                    .div_ceil(size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>())
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
            check(code)?;
            if count as usize > buffer.len() {
                return Err(Failure::new(Status::Failed, "磁盘实例数量超限"));
            }
            let mut result = BTreeMap::new();
            for item in &buffer[..count as usize] {
                // Names and values remain owned by the aligned PDH output buffer until copied.
                let name = unsafe { item.szName.to_string() }
                    .map_err(|_| Failure::new(Status::Failed, "磁盘实例名称无效"))?;
                if name == "_Total" {
                    continue;
                }
                let value = if item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA
                    || item.FmtValue.CStatus == PDH_CSTATUS_NEW_DATA
                {
                    Ok(unsafe { item.FmtValue.Anonymous.doubleValue })
                } else {
                    Err(Failure::new(Status::Warming, "磁盘实例等待有效采样"))
                };
                result.insert(name, value);
                if result.len() > pinmeter_core::disk::MAX_DISKS {
                    break;
                }
            }
            return Ok(result);
        }
        Err(Failure::new(
            Status::Failed,
            "磁盘实例发生变化，等待下次采样",
        ))
    }
}
