use std::path::Path;

pub fn missing(helper: &Path) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    return run(helper).map(|status| status == "missing");
    #[cfg(not(target_os = "windows"))]
    {
        let _ = helper;
        Ok(false)
    }
}

pub fn open_download_page() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::{
            Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
            core::w,
        };
        // A fixed publisher URL only: callers cannot provide a URL or executable.
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                w!("https://pawnio.eu/"),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            Err("无法打开浏览器，请手动访问 https://pawnio.eu/ 下载驱动".into())
        } else {
            Ok(())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("此平台不使用 PawnIO 驱动".into())
    }
}

#[cfg(target_os = "windows")]
fn run(helper: &Path) -> Result<String, String> {
    use std::{os::windows::process::CommandExt, process::Command};
    let output = Command::new(helper)
        .arg("--pawnio-status")
        .creation_flags(0x08000000)
        .output()
        .map_err(|_| "无法启动温度辅助程序，请检查完整应用目录")?;
    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|_| "温度驱动辅助程序返回无效结果")?;
    let detail = packet["detail"].as_str().ok_or("缺少温度驱动状态")?;
    if output.status.success() && packet["ok"] == true {
        Ok(detail.to_string())
    } else {
        Err(detail.to_string())
    }
}
