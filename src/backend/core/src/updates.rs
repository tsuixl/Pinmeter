use serde::{Deserialize, Serialize};

pub const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseSection {
    pub title: String,
    pub items: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseNotes {
    pub version: String,
    pub date: String,
    pub summary: String,
    pub sections: Vec<ReleaseSection>,
}
impl ReleaseNotes {
    pub fn validate(&self) -> Result<(), String> {
        semver::Version::parse(&self.version).map_err(|_| "公告版本无效")?;
        if self.summary.len() > 4096 || self.date.len() > 64 || self.sections.len() > 12 {
            return Err("公告内容超出限制".into());
        }
        for section in &self.sections {
            if section.title.len() > 120
                || section.items.len() > 60
                || section.items.iter().any(|s| s.len() > 4096)
            {
                return Err("公告内容超出限制".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatePreferences {
    pub schema_version: u32,
    pub automatic_check: bool,
    pub last_check_ms: Option<u64>,
    pub read_version: Option<String>,
    pub dismissed_version: Option<String>,
}
impl Default for UpdatePreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            automatic_check: true,
            last_check_ms: None,
            read_version: None,
            dismissed_version: None,
        }
    }
}
impl UpdatePreferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("不支持的更新设置版本".into());
        }
        for version in [&self.read_version, &self.dismissed_version]
            .into_iter()
            .flatten()
        {
            semver::Version::parse(version).map_err(|_| "更新设置中的版本无效")?;
        }
        Ok(())
    }
    pub fn due(&self, now: u64) -> bool {
        self.automatic_check
            && self
                .last_check_ms
                .is_none_or(|last| now < last || now.saturating_sub(last) >= CHECK_INTERVAL_MS)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStage {
    Idle,
    Checking,
    Available,
    Downloading,
    Ready,
    Preparing,
    Failed,
}
impl UpdateStage {
    pub fn busy(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Preparing)
    }
}
pub struct UpdateState {
    pub revision: u64,
    pub current_version: String,
    pub stage: UpdateStage,
    pub target: Option<ReleaseNotes>,
    pub target_history: Vec<ReleaseNotes>,
    pub progress: Option<u8>,
    pub detail: String,
    pub preferences: UpdatePreferences,
    pub history: Vec<ReleaseNotes>,
    pub manual_request: u64,
}
impl UpdateState {
    pub fn new(
        current: &str,
        preferences: UpdatePreferences,
        mut history: Vec<ReleaseNotes>,
    ) -> Self {
        history.retain(|r| r.validate().is_ok() && !is_newer(&r.version, current));
        history.sort_by(|a, b| {
            semver::Version::parse(&b.version)
                .unwrap()
                .cmp(&semver::Version::parse(&a.version).unwrap())
        });
        Self {
            revision: 0,
            current_version: current.into(),
            stage: UpdateStage::Idle,
            target: None,
            target_history: vec![],
            progress: None,
            detail: String::new(),
            preferences,
            history,
            manual_request: 0,
        }
    }
    pub fn change(&mut self, stage: UpdateStage, detail: impl Into<String>) {
        self.stage = stage;
        self.detail = detail.into();
        self.revision += 1;
    }
    pub fn begin_check(&mut self, manual: bool) -> Result<(), String> {
        if self.stage.busy() {
            return Err("已有更新操作正在进行".into());
        }
        if manual {
            self.manual_request += 1;
        }
        if self.stage == UpdateStage::Ready {
            self.revision += 1;
            return Ok(());
        }
        self.progress = None;
        self.change(UpdateStage::Checking, "正在检查更新…");
        Ok(())
    }
    pub fn found(
        &mut self,
        release: Option<ReleaseNotes>,
        mut history: Vec<ReleaseNotes>,
    ) -> Result<(), String> {
        if let Some(release) = release {
            release.validate()?;
            if !is_newer(&release.version, &self.current_version) {
                return Err("更新版本必须高于当前版本".into());
            }
            history.retain(|r| {
                r.validate().is_ok()
                    && is_newer(&r.version, &self.current_version)
                    && !is_newer(&r.version, &release.version)
                    && r.version != release.version
            });
            history.push(release.clone());
            history.sort_by(|a, b| {
                semver::Version::parse(&b.version)
                    .unwrap()
                    .cmp(&semver::Version::parse(&a.version).unwrap())
            });
            history.dedup_by(|a, b| a.version == b.version);
            history.truncate(5);
            self.target_history = history;
            self.target = Some(release);
            self.change(UpdateStage::Available, "发现新版本");
        } else {
            self.target = None;
            self.target_history.clear();
            self.change(UpdateStage::Idle, "当前已是最新版本");
        }
        Ok(())
    }
    pub fn begin_download(&mut self) -> Result<(), String> {
        if self.stage.busy() || self.stage == UpdateStage::Ready || self.target.is_none() {
            return Err("当前没有可下载的更新".into());
        }
        self.progress = None;
        self.change(UpdateStage::Downloading, "正在下载更新…");
        Ok(())
    }
    pub fn downloaded(&mut self) {
        self.progress = Some(100);
        self.change(UpdateStage::Ready, "更新已下载并通过签名校验，等待安装");
    }
    pub fn begin_install(&mut self) -> Result<(), String> {
        if self.stage != UpdateStage::Ready {
            return Err("更新尚未就绪".into());
        }
        self.change(UpdateStage::Preparing, "正在准备安装更新…");
        Ok(())
    }
    pub fn unread(&self) -> Vec<ReleaseNotes> {
        if self.preferences.read_version.as_deref() == Some(&self.current_version) {
            return vec![];
        }
        self.history
            .iter()
            .filter(|r| {
                self.preferences
                    .read_version
                    .as_ref()
                    .map_or(r.version == self.current_version, |read| {
                        is_newer(&r.version, read)
                    })
            })
            .take(5)
            .cloned()
            .collect()
    }
}
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(current),
    ) {
        (Ok(candidate), Ok(current)) => candidate > current,
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn notes(version: &str) -> ReleaseNotes {
        ReleaseNotes {
            version: version.into(),
            date: "2026-10-02".into(),
            summary: "更新".into(),
            sections: vec![],
        }
    }
    #[test]
    fn ready_recheck_preserves_target_and_current_version() {
        let mut state = UpdateState::new("0.1.1", Default::default(), vec![]);
        assert!(state.begin_install().is_err());
        state.begin_check(true).unwrap();
        assert!(state.begin_check(true).is_err());
        state.found(Some(notes("0.1.2")), vec![]).unwrap();
        assert!(state.begin_install().is_err());
        state.begin_download().unwrap();
        assert!(state.begin_download().is_err());
        state.downloaded();
        state.begin_check(true).unwrap();
        assert_eq!(state.stage, UpdateStage::Ready);
        assert_eq!(state.current_version, "0.1.1");
        state.begin_install().unwrap();
        assert!(state.begin_check(false).is_err());
    }
    #[test]
    fn checks_are_bounded_and_versions_are_semantic() {
        let mut p = UpdatePreferences {
            last_check_ms: Some(5000),
            ..Default::default()
        };
        assert!(!p.due(5001));
        assert!(p.due(5000 + CHECK_INTERVAL_MS));
        assert!(p.due(1));
        p.automatic_check = false;
        assert!(!p.due(u64::MAX));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.2-beta.1", "0.1.2"));
        assert!(!is_newer("garbage", "0.1.2"));
    }
    #[test]
    fn previews_do_not_mark_installed_notices_read() {
        let p = UpdatePreferences {
            read_version: Some("0.1.0".into()),
            ..Default::default()
        };
        let mut s = UpdateState::new(
            "0.1.2",
            p,
            vec![notes("0.1.1"), notes("0.1.2"), notes("0.1.3")],
        );
        assert_eq!(s.unread().len(), 2);
        s.found(Some(notes("0.1.3")), vec![]).unwrap();
        assert_eq!(s.unread().len(), 2);
        s.preferences.read_version = Some("0.1.2".into());
        assert!(s.unread().is_empty());
    }
}
