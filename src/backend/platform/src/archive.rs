use pinmeter_core::{
    app_history::{AppHistory, MAX_FILE_BYTES},
    archive::Archive,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const LIMIT: usize = 4 * 1024 * 1024;
pub struct ArchiveFile {
    path: PathBuf,
}
impl ArchiveFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<Archive, String> {
        let Some(bytes) = read_bytes(&self.path, LIMIT)? else {
            return Ok(Archive::default());
        };
        let archive: Archive =
            serde_json::from_slice(&bytes).map_err(|_| "历史文件损坏".to_string())?;
        archive.validate()?;
        Ok(archive)
    }
    pub fn preserve_invalid(&self) -> Result<(), String> {
        preserve_invalid(&self.path, LIMIT)
    }
    pub fn save(&self, archive: &Archive) -> Result<(), String> {
        archive.validate()?;
        save_bytes(
            &self.path,
            &serde_json::to_vec(archive).map_err(|e| e.to_string())?,
            LIMIT,
        )
    }
}
pub struct AppHistoryFile {
    path: PathBuf,
}
impl AppHistoryFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<AppHistory, String> {
        let Some(bytes) = read_bytes(&self.path, MAX_FILE_BYTES)? else {
            return Ok(AppHistory::default());
        };
        let archive: AppHistory =
            serde_json::from_slice(&bytes).map_err(|_| "应用历史文件损坏".to_string())?;
        archive.validate()?;
        Ok(archive)
    }
    pub fn preserve_invalid(&self) -> Result<(), String> {
        preserve_invalid(&self.path, MAX_FILE_BYTES)
    }
    pub fn save(&self, archive: &AppHistory) -> Result<(), String> {
        archive.validate()?;
        save_bytes(
            &self.path,
            &serde_json::to_vec(archive).map_err(|e| e.to_string())?,
            MAX_FILE_BYTES,
        )
    }
}
fn read_bytes(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("无法读取本地历史：{e}")),
    };
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("历史文件超过大小限制，保留原件".into());
    }
    Ok(Some(bytes))
}
fn preserve_invalid(path: &Path, limit: usize) -> Result<(), String> {
    let bytes = read_bytes(path, limit)?.ok_or("无法读取需备份的历史文件")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("history");
    for n in 0..10 {
        let backup = path.with_file_name(format!("{stem}-corrupt-{stamp}-{n}.json"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(backup)
        {
            Ok(mut file) => {
                file.write_all(&bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("无法保留损坏历史：{e}")),
        }
    }
    Err("无法创建历史备份，本次仅在内存中记录".into())
}
fn save_bytes(path: &Path, bytes: &[u8], limit: usize) -> Result<(), String> {
    if bytes.len() > limit {
        return Err("历史数据超过文件大小限制".into());
    }
    let parent = path.parent().ok_or("历史目录无效")?;
    fs::create_dir_all(parent).map_err(|e| format!("无法创建历史目录：{e}"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path)
        .map_err(|e| format!("无法原子保存历史：{}", e.error))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_file_recovers_independently_and_rejects_unbounded_or_invalid_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app-history.json");
        let file = AppHistoryFile::new(path.clone());
        assert!(file.load().unwrap().minutes.is_empty());
        let mut history = AppHistory::default();
        history.last_wall_ms = 42;
        file.save(&history).unwrap();
        assert_eq!(file.load().unwrap().last_wall_ms, 42);
        fs::write(&path, b"broken apps").unwrap();
        assert!(file.load().is_err());
        file.preserve_invalid().unwrap();
        file.save(&history).unwrap();
        let backup = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .find(|f| {
                f.file_name()
                    .to_string_lossy()
                    .starts_with("app-history-corrupt-")
            })
            .unwrap();
        assert_eq!(fs::read(backup.path()).unwrap(), b"broken apps");
        let mut large = fs::File::create(&path).unwrap();
        large.write_all(&vec![b' '; MAX_FILE_BYTES + 1]).unwrap();
        drop(large);
        assert!(file.load().is_err());
        assert!(file.preserve_invalid().is_err());
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            MAX_FILE_BYTES as u64 + 1
        );
    }
    #[test]
    fn saves_atomically_reloads_and_keeps_corrupt_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let file = ArchiveFile::new(path.clone());
        assert!(file.load().unwrap().buckets.is_empty());
        let a = Archive {
            last_wall_ms: 42,
            ..Default::default()
        };
        file.save(&a).unwrap();
        assert_eq!(file.load().unwrap().last_wall_ms, 42);
        fs::write(&path, b"broken history").unwrap();
        assert!(file.load().is_err());
        file.preserve_invalid().unwrap();
        file.save(&a).unwrap();
        let backup = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .find(|f| {
                f.file_name()
                    .to_string_lossy()
                    .starts_with("history-corrupt-")
            })
            .unwrap();
        assert_eq!(fs::read(backup.path()).unwrap(), b"broken history");
        assert_eq!(file.load().unwrap().last_wall_ms, 42);
        let bad = Archive {
            schema_version: 999,
            ..Default::default()
        };
        assert!(file.save(&bad).is_err());
        assert_eq!(file.load().unwrap().last_wall_ms, 42);
    }
}
