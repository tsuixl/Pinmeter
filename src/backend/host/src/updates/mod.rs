mod commands;
pub mod dto;
pub use commands::*;

use crate::runtime::Runtime;
use dto::*;
use pinmeter_core::updates::{ReleaseNotes, UpdatePreferences, UpdateStage, UpdateState};
use pinmeter_platform::updates::{UpdateStorage, installation_kind, read_bounded, write_atomic};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

const MAX_PACKAGE: u64 = 512 * 1024 * 1024;
const RELEASE_URL: &str = "https://github.com/tsuixl/Pinmeter/releases";
struct Pending {
    update: Update,
    checksum: Option<Vec<u8>>,
}
pub struct Updates {
    state: Mutex<UpdateState>,
    pending: Mutex<Option<Pending>>,
    preferences_writer: Mutex<()>,
    storage: UpdateStorage,
    cache: PathBuf,
    installation: &'static str,
    confirmation: std::sync::atomic::AtomicBool,
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl Updates {
    pub fn new(app: &AppHandle) -> Result<Arc<Self>, String> {
        let config = app.path().app_config_dir().map_err(|e| e.to_string())?;
        let storage = UpdateStorage::new(config.join("updates.json"));
        let loaded = storage.load();
        let diagnostic = loaded.as_ref().err().cloned();
        let history: Vec<ReleaseNotes> =
            serde_json::from_str(include_str!("../../../../shared/updates/releases.json"))
                .map_err(|e| e.to_string())?;
        let mut state = UpdateState::new(
            env!("CARGO_PKG_VERSION"),
            loaded.unwrap_or_default(),
            history,
        );
        if let Some(error) = diagnostic {
            state.detail = format!("{error}；原文件保留，更改偏好后可恢复");
        }
        let installation = installation_kind(
            &std::env::current_exe().map_err(|e| e.to_string())?,
            cfg!(debug_assertions),
        );
        Ok(Arc::new(Self {
            state: Mutex::new(state),
            pending: Mutex::new(None),
            preferences_writer: Mutex::new(()),
            storage,
            cache: app
                .path()
                .app_cache_dir()
                .map_err(|e| e.to_string())?
                .join("update-package.bin"),
            installation,
            confirmation: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    pub fn snapshot(&self) -> UpdateSnapshotDto {
        let s = self.state.lock().unwrap();
        UpdateSnapshotDto {
            revision: s.revision.to_string(),
            current_version: s.current_version.clone(),
            installation: self.installation.into(),
            can_install: self.installation == "installed",
            stage: stage_name(s.stage).into(),
            detail: s.detail.clone(),
            progress: s.progress,
            target: s.target.as_ref().map(Into::into),
            target_history: s.target_history.iter().map(Into::into).collect(),
            history: s.history.iter().map(Into::into).collect(),
            unread: s.unread().iter().map(Into::into).collect(),
            automatic_check: s.preferences.automatic_check,
            last_check_ms: s.preferences.last_check_ms.map(|v| v as f64),
            dismissed_version: s.preferences.dismissed_version.clone(),
            manual_request: s.manual_request.to_string(),
            confirmation_required: self.confirmation.load(std::sync::atomic::Ordering::Acquire),
            release_url: RELEASE_URL.into(),
        }
    }
    fn emit(&self, app: &AppHandle) {
        let _ = app.emit("pinmeter-update", self.snapshot());
    }
    fn fail(&self, app: &AppHandle, detail: String) {
        self.state
            .lock()
            .unwrap()
            .change(UpdateStage::Failed, detail);
        self.emit(app);
    }
    fn persist(&self, change: impl FnOnce(&mut UpdatePreferences)) -> Result<(), String> {
        let _writer = self.preferences_writer.lock().map_err(|e| e.to_string())?;
        let mut p = self.state.lock().unwrap().preferences.clone();
        change(&mut p);
        self.storage.save(&p)?;
        let mut s = self.state.lock().unwrap();
        s.preferences = p;
        s.revision += 1;
        Ok(())
    }
    pub fn preference(
        &self,
        app: &AppHandle,
        action: &str,
        value: Option<bool>,
    ) -> Result<(), String> {
        match action {
            "automatic" => self.persist(|p| p.automatic_check = value.unwrap_or(true))?,
            "read" => self.persist(|p| p.read_version = Some(env!("CARGO_PKG_VERSION").into()))?,
            "dismiss" => {
                let version = self
                    .state
                    .lock()
                    .unwrap()
                    .target
                    .as_ref()
                    .map(|r| r.version.clone());
                self.persist(|p| p.dismissed_version = version)?;
            }
            _ => return Err("无效的更新偏好操作".into()),
        }
        self.emit(app);
        Ok(())
    }
    pub fn start(self: &Arc<Self>, app: AppHandle) {
        let updates = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            loop {
                if updates.installation == "installed"
                    && updates.state.lock().unwrap().preferences.due(now_ms())
                {
                    let _ = updates.check(&app, false).await;
                }
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }
    pub async fn check(self: &Arc<Self>, app: &AppHandle, manual: bool) -> Result<(), String> {
        if self.installation == "unsupported" {
            return Err("此平台暂未开放在线更新".into());
        }
        {
            let mut s = self.state.lock().unwrap();
            s.begin_check(manual)?;
            if s.stage == UpdateStage::Ready {
                drop(s);
                self.emit(app);
                return Ok(());
            }
        }
        self.emit(app);
        let persist = self.persist(|p| p.last_check_ms = Some(now_ms()));
        if let Err(error) = persist {
            self.fail(app, error.clone());
            return Err(error);
        }
        let result = async {
            let mut update = app
                .updater_builder()
                .target("windows-x86_64")
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|e| e.to_string())?
                .check()
                .await
                .map_err(|e| {
                    eprintln!("Pinmeter 检查更新失败：{e}");
                    "暂时无法获取更新信息，请稍后重试或前往发行页面。".to_string()
                })?;
            let mut history = vec![];
            let notes = if let Some(u) = update.as_mut() {
                if !trusted_package_url(u.download_url.as_str()) {
                    return Err("更新包地址不属于 Pinmeter 官方发行来源".into());
                }
                u.timeout = Some(Duration::from_secs(600));
                if let Some(value) = u.raw_json.get("release_notes") {
                    history = serde_json::from_value::<Vec<ReleaseNotes>>(value.clone())
                        .map_err(|_| "更新公告格式无效")?;
                    if history.len() > 50 {
                        return Err("更新公告数量超出限制".into());
                    }
                }
                let notes =
                    history
                        .iter()
                        .find(|r| r.version == u.version)
                        .cloned()
                        .unwrap_or_else(|| ReleaseNotes {
                            version: u.version.clone(),
                            date: u.date.map(|d| d.date().to_string()).unwrap_or_default(),
                            summary: u.body.clone().filter(|s| s.len() <= 4096).unwrap_or_else(
                                || "此版本暂无详细公告，可前往发行页面查看。".into(),
                            ),
                            sections: vec![],
                        });
                Some(notes)
            } else {
                None
            };
            let mut state = self.state.lock().unwrap();
            state.found(notes, history)?;
            *self.pending.lock().unwrap() = update.map(|update| Pending {
                update,
                checksum: None,
            });
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            let detail = format!("检查更新未完成：{error}");
            self.fail(app, detail.clone());
            return Err(detail);
        }
        self.emit(app);
        Ok(())
    }
    pub async fn download(self: &Arc<Self>, app: &AppHandle) -> Result<(), String> {
        if self.installation != "installed" {
            return Err("便携版请从发行页面下载完整运行包".into());
        }
        let update = {
            let mut state = self.state.lock().unwrap();
            let pending = self.pending.lock().unwrap();
            let update = pending.as_ref().ok_or("请先检查更新")?.update.clone();
            state.begin_download()?;
            update
        };
        self.emit(app);
        let (limit_tx, mut limit_rx) = tokio::sync::mpsc::channel(1);
        let mut received = 0u64;
        let mut last = Instant::now();
        let request = update.download(
            |count, total| {
                received = received.saturating_add(count as u64);
                if received > MAX_PACKAGE || total.is_some_and(|n| n > MAX_PACKAGE) {
                    let _ = limit_tx.try_send(());
                }
                if last.elapsed() < Duration::from_millis(250) {
                    return;
                }
                last = Instant::now();
                let mut s = self.state.lock().unwrap();
                s.progress = total
                    .filter(|n| *n > 0)
                    .map(|n| ((received.saturating_mul(100) / n).min(99)) as u8);
                s.revision += 1;
                drop(s);
                self.emit(app);
            },
            || {},
        );
        let result: Result<Vec<u8>, String> = tokio::select! {
            result=request=>result.map_err(|e| {
                eprintln!("Pinmeter 下载更新失败：{e}");
                match e {
                    tauri_plugin_updater::Error::Minisign(_) | tauri_plugin_updater::Error::Base64(_) | tauri_plugin_updater::Error::SignatureUtf8(_) => "更新包校验未通过，请重新下载。".into(),
                    _ => "更新下载未完成，请检查网络后重试。".into(),
                }
            }),
            _=limit_rx.recv()=>Err("更新包超出 512 MiB 限制".into()),
        };
        let result = result.and_then(|bytes| {
            if bytes.len() as u64 > MAX_PACKAGE {
                return Err("更新包过大".into());
            }
            write_atomic(&self.cache, &bytes)?;
            let checksum = Sha256::digest(&bytes).to_vec();
            if let Some(p) = self.pending.lock().unwrap().as_mut() {
                p.checksum = Some(checksum);
            }
            Ok(())
        });
        if let Err(error) = result {
            self.fail(app, error.clone());
            return Err(error);
        }
        self.state.lock().unwrap().downloaded();
        self.emit(app);
        Ok(())
    }
    pub fn install(
        self: &Arc<Self>,
        app: &AppHandle,
        runtime: &Arc<Runtime>,
        confirmed: bool,
    ) -> Result<(), String> {
        if self.installation != "installed" {
            return Err("此运行目录不支持在线安装".into());
        }
        let (update, checksum) = {
            let mut state = self.state.lock().unwrap();
            let p = self.pending.lock().unwrap();
            let p = p.as_ref().ok_or("更新尚未就绪")?;
            let checksum = p.checksum.clone().ok_or("更新包尚未校验")?;
            state.begin_install()?;
            (p.update.clone(), checksum)
        };
        let bytes = match read_bounded(&self.cache, MAX_PACKAGE) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.fail(app, format!("无法读取已下载的更新，请重新下载：{error}"));
                return Err(error);
            }
        };
        if Sha256::digest(&bytes).as_slice() != checksum {
            self.fail(app, "缓存更新包已变化，请重新下载".into());
            return Err("更新包完整性检查失败".into());
        }
        self.confirmation
            .store(false, std::sync::atomic::Ordering::Release);
        self.emit(app);
        if let Err(error) = runtime.prepare_update(confirmed) {
            let needs_confirmation =
                matches!(error, crate::exit::PreparationError::Confirmation(_));
            let detail = error.to_string();
            self.confirmation
                .store(needs_confirmation, std::sync::atomic::Ordering::Release);
            self.state
                .lock()
                .unwrap()
                .change(UpdateStage::Ready, detail.clone());
            self.emit(app);
            return Err(detail);
        }
        runtime.stop();
        // The plugin invokes the installer then exits the process. All fallible cleanup is already complete.
        let result = update
            .install(&bytes)
            .map_err(|e| format!("无法启动安装器：{e}"));
        runtime.resume_after_failed_update(app.clone());
        let error = result
            .err()
            .unwrap_or_else(|| "安装器未完成退出交接，请重试".into());
        self.state
            .lock()
            .unwrap()
            .change(UpdateStage::Ready, error.clone());
        self.emit(app);
        Err(error)
    }
}
fn trusted_package_url(value: &str) -> bool {
    let Ok(url) = tauri::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .path()
            .starts_with("/tsuixl/Pinmeter/releases/download/")
        && url.path().ends_with(".exe")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_official_https_installer_urls_are_accepted() {
        assert!(trusted_package_url(
            "https://github.com/tsuixl/Pinmeter/releases/download/v0.1.2/Pinmeter.exe"
        ));
        for url in [
            "http://github.com/tsuixl/Pinmeter/releases/download/v1/a.exe",
            "https://github.com/other/repo/releases/download/v1/a.exe",
            "file:///C:/a.exe",
            "https://github.com.evil.test/tsuixl/Pinmeter/releases/download/v1/a.exe",
            "https://user@github.com/tsuixl/Pinmeter/releases/download/v1/a.exe",
        ] {
            assert!(!trusted_package_url(url));
        }
    }
}
