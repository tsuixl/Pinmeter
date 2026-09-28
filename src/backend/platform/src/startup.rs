//! Windows grants one token at startup; every collector inherits that token.
#[cfg(target_os = "windows")]
mod windows_startup {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt};
    use windows::{
        Win32::{
            Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, WAIT_OBJECT_0},
            Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
            System::Com::{
                COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
            },
            System::Threading::{
                GetCurrentProcess, GetExitCodeProcess, INFINITE, OpenProcess, OpenProcessToken,
                PROCESS_SYNCHRONIZE, WaitForSingleObject,
            },
            UI::{
                Shell::{
                    SEE_MASK_FLAG_NO_UI, SEE_MASK_NO_CONSOLE, SEE_MASK_NOASYNC,
                    SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
                },
                WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW, SW_SHOWNORMAL},
            },
        },
        core::{PCWSTR, w},
    };

    static LAUNCHER_EXITED: AtomicBool = AtomicBool::new(false);
    const LAUNCHER_ARG: &str = "--pinmeter-launcher=";

    pub fn launcher_exited() -> bool {
        LAUNCHER_EXITED.load(Ordering::Relaxed)
    }

    fn watch_launcher(pid: u32) -> Result<(), String> {
        let handle =
            unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.map_err(|e| e.to_string())?;
        let raw = handle.0 as usize;
        std::thread::spawn(move || {
            let process = OwnedHandle(HANDLE(raw as *mut _));
            if unsafe { WaitForSingleObject(process.0, INFINITE) } == WAIT_OBJECT_0 {
                LAUNCHER_EXITED.store(true, Ordering::Relaxed);
            }
        });
        Ok(())
    }

    struct OwnedHandle(HANDLE);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }

    // Match Windows argv rules, including quotes and trailing backslashes.
    fn quote(value: &OsStr) -> Vec<u16> {
        let mut result = vec![34];
        let mut slashes = 0;
        for unit in value.encode_wide() {
            if unit == 92 {
                slashes += 1;
                continue;
            }
            result.extend(std::iter::repeat_n(
                92,
                slashes * if unit == 34 { 2 } else { 1 },
            ));
            slashes = 0;
            if unit == 34 {
                result.push(92);
            }
            result.push(unit);
        }
        result.extend(std::iter::repeat_n(92, slashes * 2));
        result.push(34);
        result
    }

    pub fn prepare() -> Result<Option<u32>, String> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(|error| error.to_string())?;
        let token = OwnedHandle(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut length = 0;
        unsafe {
            GetTokenInformation(
                token.0,
                TokenElevation,
                Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut length,
            )
        }
        .map_err(|error| error.to_string())?;
        let launcher = std::env::args_os().skip(1).find_map(|arg| {
            arg.to_str()?
                .strip_prefix(LAUNCHER_ARG)?
                .parse::<u32>()
                .ok()
        });
        if elevation.TokenIsElevated != 0 {
            if let Some(pid) = launcher {
                watch_launcher(pid)?;
            }
            return Ok(None);
        }
        if launcher.is_some() {
            return Err("系统未授予管理员权限，请重新启动 Pinmeter".into());
        }

        let executable = wide(
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .as_os_str(),
        );
        let directory = wide(
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .as_os_str(),
        );
        let mut parameters = Vec::new();
        for arg in std::env::args_os().skip(1) {
            if !parameters.is_empty() {
                parameters.push(32);
            }
            parameters.extend(quote(&arg));
        }
        if !parameters.is_empty() {
            parameters.push(32);
        }
        parameters.extend(quote(OsStr::new(&format!(
            "{LAUNCHER_ARG}{}",
            std::process::id()
        ))));
        parameters.push(0);
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
            .ok()
            .map_err(|e| e.to_string())?;
        struct ComGuard;
        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _com = ComGuard;
        let mut launch = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS
                | SEE_MASK_NOASYNC
                | SEE_MASK_FLAG_NO_UI
                | SEE_MASK_NO_CONSOLE,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            lpDirectory: PCWSTR(directory.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        if let Err(error) = unsafe { ShellExecuteExW(&mut launch) } {
            if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(Some(0));
            }
            return Err(error.to_string());
        }
        let process = OwnedHandle(launch.hProcess);
        // Keep cargo/tauri dev attached until the actual desktop instance exits.
        if unsafe { WaitForSingleObject(process.0, INFINITE) } != WAIT_OBJECT_0 {
            return Err("无法等待已授权的 Pinmeter 进程".into());
        }
        let mut code = 0;
        unsafe { GetExitCodeProcess(process.0, &mut code) }.map_err(|e| e.to_string())?;
        Ok(Some(code))
    }

    pub fn report_error(error: &str) {
        let message = wide(OsStr::new(&format!("Pinmeter 启动授权失败：{error}")));
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(message.as_ptr()),
                w!("Pinmeter"),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};

        #[test]
        fn elevation_arguments_round_trip_through_windows_parser() {
            let args = [
                "",
                "two words",
                "中文目录\\",
                "embedded\"quote",
                "slash\\\"quote",
            ];
            let mut command = quote(OsStr::new("pinmeter.exe"));
            for arg in args {
                command.push(32);
                command.extend(quote(OsStr::new(arg)));
            }
            command.push(0);
            let mut count = 0;
            unsafe {
                let parsed = CommandLineToArgvW(PCWSTR(command.as_ptr()), &mut count);
                assert!(!parsed.is_null());
                assert_eq!(count as usize, args.len() + 1);
                for (index, expected) in args.iter().enumerate() {
                    assert_eq!((*parsed.add(index + 1)).to_string().unwrap(), *expected);
                }
                let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(parsed.cast())));
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_startup::{launcher_exited, prepare, report_error};

#[cfg(not(target_os = "windows"))]
pub fn launcher_exited() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn prepare() -> Result<Option<u32>, String> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn report_error(error: &str) {
    eprintln!("Pinmeter: {error}");
}
