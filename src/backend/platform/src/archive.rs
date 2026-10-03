use pinmeter_core::{
    app_history::{AppHistory, MAX_FILE_BYTES},
    archive::{Archive, MAX_FILE_BYTES as MAX_ARCHIVE_FILE_BYTES},
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub struct ArchiveFile {
    path: PathBuf,
}
impl ArchiveFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<Archive, String> {
        let Some(bytes) = read_bytes(&self.path, MAX_ARCHIVE_FILE_BYTES)? else {
            return Ok(Archive::default());
        };
        let mut archive: Archive =
            serde_json::from_slice(&bytes).map_err(|_| "历史文件损坏".to_string())?;
        archive.validate()?;
        archive.migrate();
        archive.validate()?;
        Ok(archive)
    }
    pub fn preserve_invalid(&self) -> Result<(), String> {
        preserve_invalid(&self.path, MAX_ARCHIVE_FILE_BYTES)
    }
    pub fn save(&self, archive: &Archive) -> Result<(), String> {
        archive.validate()?;
        save_bytes(
            &self.path,
            &serde_json::to_vec(archive).map_err(|e| e.to_string())?,
            MAX_ARCHIVE_FILE_BYTES,
        )
    }
    pub fn storage_bytes(&self) -> Result<u64, String> {
        storage_bytes(&self.path)
    }
    pub fn clear(&self, archive: &Archive) -> Result<(), String> {
        remove_managed_backups(&self.path)?;
        self.save(archive)
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
        let mut archive: AppHistory =
            serde_json::from_slice(&bytes).map_err(|_| "应用历史文件损坏".to_string())?;
        archive.validate()?;
        archive.migrate();
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
    pub fn storage_bytes(&self) -> Result<u64, String> {
        storage_bytes(&self.path)
    }
    pub fn clear(&self, history: &AppHistory) -> Result<(), String> {
        remove_managed_backups(&self.path)?;
        self.save(history)
    }
}

fn managed_backups(path: &Path) -> Result<Vec<PathBuf>, String> {
    let parent = path.parent().ok_or("历史目录无效")?;
    let prefix = format!(
        "{}-corrupt-",
        path.file_stem()
            .and_then(|v| v.to_str())
            .ok_or("历史名称无效")?
    );
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error.to_string()),
    };
    let mut result = vec![];
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(suffix) = name
            .to_str()
            .and_then(|n| n.strip_prefix(&prefix))
            .and_then(|n| n.strip_suffix(".json"))
        else {
            continue;
        };
        let parts: Vec<_> = suffix.split('-').collect();
        if parts.len() == 2
            && parts
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            && entry.file_type().map_err(|e| e.to_string())?.is_file()
        {
            result.push(entry.path());
        }
    }
    Ok(result)
}
fn remove_managed_backups(path: &Path) -> Result<(), String> {
    for backup in managed_backups(path)? {
        fs::remove_file(backup).map_err(|e| format!("无法清除损坏备份：{e}"))?;
    }
    Ok(())
}
fn storage_bytes(path: &Path) -> Result<u64, String> {
    let mut bytes = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error.to_string()),
    };
    for backup in managed_backups(path)? {
        bytes = bytes.saturating_add(fs::metadata(backup).map_err(|e| e.to_string())?.len());
    }
    Ok(bytes)
}

pub fn export_csv(
    directory: &Path,
    scope: &str,
    content: &str,
    at_ms: u64,
) -> Result<PathBuf, String> {
    if !matches!(scope, "basic" | "applications") || content.len() > 32 * 1024 * 1024 {
        return Err("历史导出范围无效或内容过大".into());
    }
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    for serial in 0..100 {
        let path = directory.join(format!("Pinmeter-{scope}-{at_ms}-{serial}.csv"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(b"\xef\xbb\xbf")
                    .and_then(|_| file.write_all(content.as_bytes()))
                    .and_then(|_| file.sync_all())
                    .map_err(|e| format!("导出未完成：{e}"))?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("无法建立新的历史导出文件".into())
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
    fn complete_escaped_archive_fits_the_shared_budget_and_atomically_round_trips() {
        use pinmeter_core::archive::{
            Aggregate, HOUR, MAX_BUCKETS, MAX_HOURS, MAX_NETWORK_TEXT_BYTES, MAX_NETWORKS,
            MAX_QUARTERS, MAX_SERIALIZED_BYTES_BOUND, MINUTE, MinuteBucket, NetworkSource, Peak,
            QUARTER_HOUR,
        };
        let mut archive = Archive {
            last_wall_ms: u64::MAX,
            clock_discontinuities: u64::MAX,
            cleared_before_ms: u64::MAX,
            ..Default::default()
        };
        // Control characters are legal under the existing 512-byte limits and
        // each serializes as six bytes (\u0000 etc), unlike ordinary UTF-8 text.
        let sources: Vec<_> = (0..MAX_NETWORKS)
            .map(|index| NetworkSource {
                id: char::from_u32(index as u32)
                    .unwrap()
                    .to_string()
                    .repeat(MAX_NETWORK_TEXT_BYTES),
                name: "\u{1}".repeat(MAX_NETWORK_TEXT_BYTES),
            })
            .collect();
        for (resolution_ms, count) in [
            (MINUTE, MAX_BUCKETS),
            (QUARTER_HOUR, MAX_QUARTERS),
            (HOUR, MAX_HOURS),
        ] {
            let aggregate = Aggregate {
                weighted_sum: resolution_ms as f64 * 98.76543210987654,
                covered_ms: resolution_ms,
                min: Some(f64::MIN_POSITIVE),
                max: Some(99.99999999999999),
                max_at_ms: Some(u64::MAX),
            };
            let bucket = MinuteBucket {
                cpu: aggregate.clone(),
                memory: aggregate,
                received: u64::MAX,
                transmitted: u64::MAX,
                network_ms: resolution_ms,
                networks: sources.clone(),
                multiple_networks: true,
                download_peak: Some(Peak {
                    value: f64::MAX,
                    at_ms: u64::MAX,
                }),
                upload_peak: Some(Peak {
                    value: f64::MAX,
                    at_ms: u64::MAX,
                }),
            };
            let latest = archive.last_wall_ms / resolution_ms * resolution_ms;
            let layer = match resolution_ms {
                MINUTE => &mut archive.buckets,
                QUARTER_HOUR => &mut archive.quarters,
                _ => &mut archive.hours,
            };
            for index in 0..count {
                layer.insert(latest - index as u64 * resolution_ms, bucket.clone());
            }
        }
        archive.validate().unwrap();
        let mut chinese_names = archive.clone();
        let ordinary_sources: Vec<_> = (0..MAX_NETWORKS)
            .map(|index| NetworkSource {
                id: format!("network-{index}"),
                name: "网".repeat(128),
            })
            .collect();
        for bucket in chinese_names
            .buckets
            .values_mut()
            .chain(chinese_names.quarters.values_mut())
            .chain(chinese_names.hours.values_mut())
        {
            bucket.networks = ordinary_sources.clone();
        }
        chinese_names.validate().unwrap();
        let ordinary_length = serde_json::to_vec(&chinese_names).unwrap().len();
        assert!(
            ordinary_length > 4 * 1024 * 1024,
            "ordinary Chinese names already exceed the previous file limit"
        );
        println!("基础历史完整三层128中文网卡名：{ordinary_length} 字节");
        drop(chinese_names);
        let encoded = serde_json::to_vec(&archive).unwrap();
        let length = encoded.len();
        assert!(
            length > 64 * 1024 * 1024,
            "this fixture must exercise the escaped-string worst case"
        );
        assert!(length <= MAX_SERIALIZED_BYTES_BOUND);
        assert!(length < MAX_ARCHIVE_FILE_BYTES);
        println!(
            "基础历史完整转义极值：{length} 字节；形式上界 {MAX_SERIALIZED_BYTES_BOUND}；文件硬上限 {MAX_ARCHIVE_FILE_BYTES}"
        );
        drop(encoded);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let store = ArchiveFile::new(path.clone());
        store.save(&Archive::default()).unwrap();
        store.save(&archive).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), length as u64);
        let restored = store.load().unwrap();
        restored.validate().unwrap();
        assert_eq!(
            (
                restored.buckets.len(),
                restored.quarters.len(),
                restored.hours.len()
            ),
            (MAX_BUCKETS, MAX_QUARTERS, MAX_HOURS)
        );
        for resolution_ms in [MINUTE, QUARTER_HOUR, HOUR] {
            let bucket = restored.layer(resolution_ms).last_key_value().unwrap().1;
            assert_eq!(bucket.networks, sources);
            assert_eq!(bucket.received, u64::MAX);
            assert_eq!(bucket.cpu.max_at_ms, Some(u64::MAX));
            assert_eq!(bucket.download_peak.as_ref().unwrap().value, f64::MAX);
        }
        // A successful replace leaves only the destination, with no temp copy.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn clear_removes_only_matching_regular_backups_and_preserves_exports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let file = ArchiveFile::new(path.clone());
        file.save(&Archive::default()).unwrap();
        fs::write(dir.path().join("history-corrupt-123-0.json"), "backup").unwrap();
        fs::write(dir.path().join("history-corrupt-not-owned.json"), "keep").unwrap();
        fs::write(dir.path().join("app-history-corrupt-123-0.json"), "keep").unwrap();
        fs::write(dir.path().join("history.csv"), "keep").unwrap();
        fs::create_dir(dir.path().join("history-corrupt-123-1.json")).unwrap();
        file.clear(&Archive::cleared(1000)).unwrap();
        assert!(!dir.path().join("history-corrupt-123-0.json").exists());
        for keep in [
            "history-corrupt-not-owned.json",
            "app-history-corrupt-123-0.json",
            "history.csv",
            "history-corrupt-123-1.json",
        ] {
            assert!(dir.path().join(keep).exists());
        }
        assert_eq!(file.load().unwrap().cleared_before_ms, 1000);
    }
    #[test]
    fn csv_export_uses_a_new_file_and_preserves_utf8_and_exact_integers() {
        let dir = tempfile::tempdir().unwrap();
        let content = "应用,字节\r\n测试,18446744073709551615\r\n";
        let first = export_csv(dir.path(), "applications", content, 1000).unwrap();
        let second = export_csv(dir.path(), "applications", content, 1000).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            fs::read_to_string(first).unwrap(),
            format!("\u{feff}{content}")
        );
    }
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
