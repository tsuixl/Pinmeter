use pinmeter_core::updates::UpdatePreferences;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct UpdateStorage {
    path: PathBuf,
}
impl UpdateStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<UpdatePreferences, String> {
        if !self.path.exists() {
            return Ok(UpdatePreferences::default());
        }
        let bytes = read_bounded(&self.path, 65536)?;
        let p: UpdatePreferences =
            serde_json::from_slice(&bytes).map_err(|e| format!("更新设置损坏：{e}"))?;
        p.validate()?;
        Ok(p)
    }
    pub fn save(&self, p: &UpdatePreferences) -> Result<(), String> {
        p.validate()?;
        if self.path.exists() && self.load().is_err() {
            fs::copy(&self.path, self.path.with_extension("corrupt.json"))
                .map_err(|e| e.to_string())?;
        }
        write_atomic(
            &self.path,
            &serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?,
        )
    }
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("更新路径无效")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist(path)
        .map_err(|e| format!("无法保存更新文件：{}", e.error))?;
    Ok(())
}
pub fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut data = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > max {
        return Err("更新文件超出大小限制".into());
    }
    Ok(data)
}
/// An NSIS marker must match this exact executable. Moving a directory makes it portable.
pub fn installation_kind(exe: &Path, development: bool) -> &'static str {
    if development {
        return "development";
    }
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return "unsupported";
    }
    let Some(parent) = exe.parent() else {
        return "portable";
    };
    if parent.join("source-manifest.json").exists() || parent.join("build-source.json").exists() {
        return "portable";
    }
    let Ok(marker) = read_bounded(&parent.join(".pinmeter-installed"), 16384) else {
        return "portable";
    };
    let value = if marker.starts_with(&[0xff, 0xfe]) {
        let words: Vec<u16> = marker[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16_lossy(&words)
    } else {
        String::from_utf8_lossy(&marker).into_owned()
    };
    let Ok(installed) = Path::new(value.trim()).canonicalize() else {
        return "portable";
    };
    if exe.canonicalize().is_ok_and(|current| {
        current
            .to_string_lossy()
            .eq_ignore_ascii_case(&installed.to_string_lossy())
    }) {
        "installed"
    } else {
        "portable"
    }
}
pub fn open_downloads() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::{
            Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
            core::{HSTRING, w},
        };
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                &HSTRING::from("https://github.com/tsuixl/Pinmeter/releases"),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            Err("无法打开默认浏览器".into())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        Err("请在浏览器打开 https://github.com/tsuixl/Pinmeter/releases".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_preferences_are_preserved_when_repaired() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("updates.json");
        let store = UpdateStorage::new(path.clone());
        assert!(store.load().unwrap().automatic_check);
        store
            .save(&UpdatePreferences {
                automatic_check: false,
                ..Default::default()
            })
            .unwrap();
        assert!(!store.load().unwrap().automatic_check);
        fs::write(&path, "broken").unwrap();
        assert!(store.load().is_err());
        store.save(&Default::default()).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("corrupt.json")).unwrap(),
            "broken"
        );
    }
    #[test]
    fn test_packages_cannot_authorize_online_installation() {
        let d = tempfile::tempdir().unwrap();
        let exe = d.path().join("Pinmeter.exe");
        fs::write(&exe, "").unwrap();
        assert_eq!(installation_kind(&exe, true), "development");
        assert_ne!(installation_kind(&exe, false), "installed");
        fs::write(
            d.path().join(".pinmeter-installed"),
            exe.to_string_lossy().as_bytes(),
        )
        .unwrap();
        if cfg!(windows) {
            assert_eq!(installation_kind(&exe, false), "installed");
        }
        fs::write(d.path().join("source-manifest.json"), "{}").unwrap();
        assert_ne!(installation_kind(&exe, false), "installed");
    }
}
