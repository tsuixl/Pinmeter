use std::path::Path;

pub fn missing(helper: &Path) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    return run(helper, false).map(|status| status == "missing");
    #[cfg(not(target_os = "windows"))]
    {
        let _ = helper;
        Ok(false)
    }
}

pub fn install(helper: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    return run(helper, true);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = helper;
        Err("此平台不使用 PawnIO 驱动".into())
    }
}

#[cfg(target_os = "windows")]
fn run(helper: &Path, install: bool) -> Result<String, String> {
    use std::{os::windows::process::CommandExt, process::Command, sync::Mutex};
    // Covers the full process lifetime; duplicate requests cannot launch installers.
    static INSTALL: Mutex<()> = Mutex::new(());
    let _guard = if install {
        Some(INSTALL.try_lock().map_err(|_| "PawnIO 正在安装，请稍候")?)
    } else {
        None
    };
    let output = Command::new(helper)
        .arg(if install {
            "--install-pawnio"
        } else {
            "--pawnio-status"
        })
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
