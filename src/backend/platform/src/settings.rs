use pinmeter_core::{domain::Settings, ports::SettingsRepository};
use std::{fs, io::Write, path::PathBuf};

pub struct FileSettings {
    path: PathBuf,
}
impl FileSettings {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
impl SettingsRepository for FileSettings {
    fn load(&self) -> Result<Option<Settings>, String> {
        if !self.path.exists() {
            return Ok(None);
        }
        if fs::metadata(&self.path).map_err(|e| e.to_string())?.len() > 65536 {
            return Err("配置文件过大".into());
        }
        let bytes = fs::read(&self.path).map_err(|e| e.to_string())?;
        let settings: Settings =
            serde_json::from_slice(&bytes).map_err(|e| format!("配置损坏：{e}"))?;
        settings.validate()?;
        Ok(Some(settings))
    }
    fn save(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let parent = self.path.parent().ok_or("配置目录无效")?;
        fs::create_dir_all(parent).map_err(|e| format!("无法创建配置目录：{e}"))?;
        // Keep a diagnostic copy only when replacing an invalid configuration.
        if self.path.exists() && self.load().is_err() {
            fs::copy(&self.path, self.path.with_extension("corrupt.json"))
                .map_err(|e| format!("无法保留损坏配置：{e}"))?;
        }
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(&serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(&self.path)
            .map_err(|e| format!("无法原子保存配置：{}", e.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_settings_default_to_release_and_explicit_opt_out_persists() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, br#"{"schema_version":1,"revision":2,"theme":"dark","interval_ms":1000,"network_id":null,"taskbar":{"enabled":false,"hidden":false,"layout":"double","cpu":true,"memory":true}}"#).unwrap();
        let repository = FileSettings::new(path);
        let mut settings = repository.load().unwrap().unwrap();
        assert!(!settings.autostart);
        settings.autostart = true;
        repository.save(&settings).unwrap();
        assert!(repository.load().unwrap().unwrap().autostart);
        settings.autostart = false;
        repository.save(&settings).unwrap();
        assert!(!repository.load().unwrap().unwrap().autostart);
        assert_eq!(settings.close_action, "ask");
        for action in ["minimize", "exit", "ask"] {
            settings.close_action = action.into();
            repository.save(&settings).unwrap();
            assert_eq!(repository.load().unwrap().unwrap().close_action, action);
        }
        settings.close_action = "unknown".into();
        assert!(repository.save(&settings).is_err());
        assert_eq!(repository.load().unwrap().unwrap().close_action, "ask");
        settings.close_action = "ask".into();
        assert!(settings.release_network_on_exit);
        assert_eq!(
            settings.taskbar,
            pinmeter_core::desktop::TaskbarSettings::default()
        );
        settings.taskbar.enabled = true;
        settings.taskbar.gpu = false;
        settings.taskbar.hidden = true;
        settings.taskbar.layout = "single".into();
        settings.release_network_on_exit = false;
        repository.save(&settings).unwrap();
        assert!(!repository.load().unwrap().unwrap().release_network_on_exit);
        assert_eq!(
            repository.load().unwrap().unwrap().taskbar,
            settings.taskbar
        );
        settings.taskbar.layout = "unknown".into();
        assert!(repository.save(&settings).is_err());
        assert_eq!(repository.load().unwrap().unwrap().taskbar.layout, "single");
    }
    #[test]
    fn saves_replaces_and_recovers_corruption_without_losing_original() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let repository = FileSettings::new(path.clone());
        assert!(repository.load().unwrap().is_none());
        repository.save(&Settings::default()).unwrap();
        let settings = Settings {
            revision: 1,
            theme: "dark".into(),
            ..Default::default()
        };
        repository.save(&settings).unwrap();
        assert_eq!(repository.load().unwrap().unwrap(), settings);
        fs::write(&path, b"broken-json").unwrap();
        assert!(repository.load().is_err());
        repository.save(&settings).unwrap();
        assert_eq!(
            fs::read(path.with_extension("corrupt.json")).unwrap(),
            b"broken-json"
        );
    }
    #[test]
    fn filesystem_failure_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("blocked");
        fs::write(&parent, b"file").unwrap();
        let repository = FileSettings::new(parent.join("settings.json"));
        assert!(repository.save(&Settings::default()).is_err());
    }
}
