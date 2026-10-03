use pinmeter_core::diagnostics::DiagnosticSystem;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

pub fn system() -> DiagnosticSystem {
    #[cfg(windows)]
    let version = {
        use windows::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        unsafe { GetVersionExW(&mut info) }.ok().map(|_| {
            format!(
                "{}.{}.{}",
                info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
            )
        })
    };
    #[cfg(not(windows))]
    let version = None;
    DiagnosticSystem {
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        version,
    }
}
pub fn export(directory: &Path, content: &str, created_at: u64) -> Result<PathBuf, String> {
    if content.len() > 65536 {
        return Err("诊断内容超出限制".into());
    }
    if !directory.is_dir() {
        return Err("下载目录不可用".into());
    }
    for suffix in 1..=100 {
        let path = directory.join(format!("Pinmeter-diagnostics-{created_at}-{suffix}.json"));
        let mut file = match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("无法创建诊断文件：{error}")),
        };
        if let Err(error) = file
            .write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
        {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(format!("无法写入诊断文件：{error}"));
        }
        return Ok(path);
    }
    Err("同名诊断文件过多，请稍后重试".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_exact_utf8_preview_and_never_overwrites_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let content = "{\"状态\":\"采样中\"}\n";
        let first = export(directory.path(), content, 1).unwrap();
        let second = export(directory.path(), "{}", 1).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read_to_string(first).unwrap(), content);
        assert!(export(&directory.path().join("missing"), "{}", 1).is_err());
        assert!(export(directory.path(), &"x".repeat(65537), 1).is_err());
    }
}
