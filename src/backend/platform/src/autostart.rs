use pinmeter_core::autostart::{Autostart, AutostartStatus};

pub struct SystemAutostart;

impl Autostart for SystemAutostart {
    fn status(&self) -> AutostartStatus {
        #[cfg(target_os = "windows")]
        match windows::run("status", "") {
            Ok(value) => AutostartStatus {
                available: true,
                enabled: Some(matches!(value.trim(), "enabled" | "other_path")),
                detail: match value.trim() {
                    "enabled" => "已开启，登录 Windows 后自动启动".into(),
                    "other_path" => "启动项指向其他位置，请在此版本关闭后重新开启".into(),
                    _ => "未开启开机自启".into(),
                },
            },
            Err(error) => AutostartStatus {
                available: true,
                enabled: None,
                detail: error,
            },
        }
        #[cfg(not(target_os = "windows"))]
        AutostartStatus {
            available: false,
            enabled: Some(false),
            detail: "当前平台尚不支持开机自启".into(),
        }
    }

    fn checkpoint(&self) -> Result<String, String> {
        #[cfg(target_os = "windows")]
        return windows::run("checkpoint", "");
        #[cfg(not(target_os = "windows"))]
        Ok(String::new())
    }

    fn restore(&self, checkpoint: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        return windows::run("restore", checkpoint).map(|_| ());
        #[cfg(not(target_os = "windows"))]
        {
            let _ = checkpoint;
            Ok(())
        }
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        return windows::run(if enabled { "enable" } else { "disable" }, "").map(|_| ());
        #[cfg(not(target_os = "windows"))]
        if enabled {
            Err("当前平台尚不支持开机自启".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{
        io::{Read, Write},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    pub fn run(action: &str, checkpoint: &str) -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let system = std::env::var_os("SystemRoot").ok_or("无法定位 Windows 目录")?;
        // Executable paths are environment data, never interpolated into PowerShell code.
        let mut child = Command::new(
            std::path::Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        )
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "-"])
        .env("PINMETER_AUTOSTART_ACTION", action)
        .env("PINMETER_AUTOSTART_EXE", &exe)
        .env("PINMETER_AUTOSTART_CHECKPOINT", checkpoint)
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法管理开机自启：{e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let input = stdin
            .write_all(include_bytes!("autostart.ps1"))
            .and_then(|_| stdin.write_all(b"\n"));
        drop(stdin);
        if let Err(error) = input {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("无法管理开机自启：{error}"));
        }
        // Drain both pipes while waiting: an XML checkpoint can exceed pipe capacity.
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let out = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let err = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        let start = Instant::now();
        loop {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                break;
            }
            if start.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out.join();
                let _ = err.join();
                return Err("系统启动项操作超时，请重试并检查任务计划程序".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let output = out
            .join()
            .map_err(|_| "启动项回执读取失败")?
            .map_err(|e| e.to_string())?;
        let error = err
            .join()
            .map_err(|_| "启动项错误读取失败")?
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!(
                "无法管理开机自启：{}",
                String::from_utf8_lossy(&error).trim()
            ));
        }
        let value = String::from_utf8_lossy(&output).trim().to_owned();
        if action != "checkpoint"
            && !matches!(value.as_str(), "enabled" | "disabled" | "other_path" | "ok")
        {
            return Err("系统未确认启动项操作结果，请重试".into());
        }
        Ok(value)
    }
}
