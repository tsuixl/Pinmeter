use pinmeter_core::archive::Archive;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};
const LIMIT: u64 = 4 * 1024 * 1024;
pub struct ArchiveFile {
    path: PathBuf,
}
impl ArchiveFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<Archive, String> {
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Archive::default()),
            Err(e) => return Err(format!("无法读取本地历史：{e}")),
        };
        let mut bytes = Vec::new();
        file.take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("历史文件超过 4 MiB 限制".into());
        }
        let archive: Archive =
            serde_json::from_slice(&bytes).map_err(|_| "历史文件损坏".to_string())?;
        archive.validate()?;
        Ok(archive)
    }
    pub fn preserve_invalid(&self) -> Result<(), String> {
        let mut bytes = Vec::new();
        fs::File::open(&self.path)
            .map_err(|e| e.to_string())?
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("历史文件过大，保留原位，本次仅在内存中记录".into());
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for n in 0..10 {
            let backup = self
                .path
                .with_file_name(format!("history-corrupt-{stamp}-{n}.json"));
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
    pub fn save(&self, archive: &Archive) -> Result<(), String> {
        archive.validate()?;
        let bytes = serde_json::to_vec(archive).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("历史数据超过文件大小限制".into());
        }
        let parent = self.path.parent().ok_or("历史目录无效")?;
        fs::create_dir_all(parent).map_err(|e| format!("无法创建历史目录：{e}"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(&self.path)
            .map_err(|e| format!("无法原子保存历史：{}", e.error))?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
