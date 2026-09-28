use pinmeter_core::hardware::HardwareState;

pub fn collect(cancel: &std::sync::atomic::AtomicBool) -> HardwareState {
    #[cfg(target_os = "windows")]
    return match windows_inventory(cancel) {
        Ok(mut inventory) => {
            inventory.normalize();
            HardwareState::Ready(inventory)
        }
        Err(error) => HardwareState::Failed(error),
    };
    #[cfg(not(target_os = "windows"))]
    {
        let _ = cancel;
        HardwareState::Unsupported
    }
}

#[cfg(target_os = "windows")]
fn windows_inventory(
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<pinmeter_core::hardware::HardwareInventory, String> {
    use std::{
        io::Read,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    // Fixed, embedded script only. No profile, user arguments, or on-disk script dependency.
    let system_root = std::env::var_os("SystemRoot").ok_or("未找到 Windows 系统目录")?;
    let executable = std::path::PathBuf::from(system_root)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("hardware/windows.ps1"),
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "无法启动系统硬件查询")?;
    let stdout = child.stdout.take().ok_or("无法读取硬件查询结果")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let started = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err("系统硬件查询失败")
                };
            }
            Ok(None)
                if started.elapsed() < Duration::from_secs(40)
                    && !cancel.load(std::sync::atomic::Ordering::Relaxed) =>
            {
                std::thread::sleep(Duration::from_millis(100))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("硬件查询超时或进程不可用，请重新启动应用重试");
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "硬件结果读取线程失败")?
        .map_err(|_| "硬件结果读取失败")?;
    result?;
    if bytes.len() > 1_048_576 {
        return Err("硬件清单超出大小限制".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "系统返回的硬件信息格式无效".into())
}
