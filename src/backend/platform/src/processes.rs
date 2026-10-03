use pinmeter_core::{
    domain::{Failure, Status},
    processes::ProcessBatch,
};
#[cfg(not(target_os = "windows"))]
pub fn collect() -> Result<ProcessBatch, Failure> {
    Err(Failure::new(Status::Unsupported, "此平台尚未支持进程排行"))
}
#[cfg(target_os = "windows")]
pub fn collect() -> Result<ProcessBatch, Failure> {
    native::collect()
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use pinmeter_core::processes::{MAX_PROCESSES, ProcessSample};
    use windows::{
        Win32::{
            Foundation::{CloseHandle, ERROR_NO_MORE_FILES, FILETIME, HANDLE},
            System::{Diagnostics::ToolHelp::*, ProcessStatus::*, Threading::*},
        },
        core::{Error, PWSTR},
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    fn error(e: Error) -> Failure {
        Failure::new(
            if e.code().0 as u32 == 0x80070005 {
                Status::PermissionDenied
            } else {
                Status::Failed
            },
            "进程已退出或系统限制访问",
        )
    }
    fn filetime(t: FILETIME) -> u64 {
        ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
    }
    pub fn collect() -> Result<ProcessBatch, Failure> {
        let snapshot =
            Handle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(error)?);
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut next = unsafe { Process32FirstW(snapshot.0, &mut entry) };
        let mut rows = Vec::new();
        let mut truncated = false;
        loop {
            if let Err(e) = next {
                if e.code() == ERROR_NO_MORE_FILES.to_hresult() {
                    break;
                }
                return Err(error(e));
            }
            if entry.th32ProcessID != 0 {
                if rows.len() >= MAX_PROCESSES {
                    truncated = true;
                    break;
                }
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|v| *v == 0)
                    .unwrap_or(entry.szExeFile.len());
                let mut name = String::from_utf16_lossy(&entry.szExeFile[..length]);
                let (times, working_set) = match unsafe {
                    OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION,
                        false,
                        entry.th32ProcessID,
                    )
                } {
                    Ok(handle) => {
                        let handle = Handle(handle);
                        let mut path = [0u16; 4096];
                        let mut len = path.len() as u32;
                        if unsafe {
                            QueryFullProcessImageNameW(
                                handle.0,
                                PROCESS_NAME_WIN32,
                                PWSTR(path.as_mut_ptr()),
                                &mut len,
                            )
                        }
                        .is_ok()
                        {
                            // Keep only the file name, never its path or command line.
                            if let Some(file) = String::from_utf16_lossy(&path[..len as usize])
                                .rsplit('\\')
                                .next()
                            {
                                name = file.into();
                            }
                        }
                        let (mut birth, mut exit, mut kernel, mut user) = (
                            FILETIME::default(),
                            FILETIME::default(),
                            FILETIME::default(),
                            FILETIME::default(),
                        );
                        let times = unsafe {
                            GetProcessTimes(handle.0, &mut birth, &mut exit, &mut kernel, &mut user)
                        }
                        .map_err(error)
                        .and_then(|_| {
                            filetime(kernel)
                                .checked_add(filetime(user))
                                .map(|total| (filetime(birth), total))
                                .ok_or_else(|| Failure::new(Status::Failed, "进程时间超限"))
                        });
                        let mut memory = PROCESS_MEMORY_COUNTERS {
                            cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                            ..Default::default()
                        };
                        let working_set = unsafe {
                            GetProcessMemoryInfo(
                                handle.0,
                                &mut memory,
                                size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                            )
                        }
                        .map(|_| memory.WorkingSetSize as u64)
                        .map_err(error);
                        (times, working_set)
                    }
                    Err(e) => {
                        let e = error(e);
                        (Err(e.clone()), Err(e))
                    }
                };
                rows.push(ProcessSample {
                    pid: entry.th32ProcessID,
                    name,
                    times,
                    working_set,
                });
            }
            next = unsafe { Process32NextW(snapshot.0, &mut entry) };
        }
        let logical_cpus = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
        Ok(ProcessBatch {
            rows,
            logical_cpus,
            truncated,
        })
    }
}
