use crate::runtime::Runtime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

pub(crate) enum PreparationError {
    Confirmation(String),
    Failed(String),
}
impl std::fmt::Display for PreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Confirmation(s) | Self::Failed(s) => f.write_str(s),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ExitStatusDto {
    pub stage: String,
    pub detail: String,
}
impl Default for ExitStatusDto {
    fn default() -> Self {
        Self {
            stage: "idle".into(),
            detail: String::new(),
        }
    }
}
impl Runtime {
    pub fn minimize_to_tray(&self, app: &AppHandle) -> Result<(), String> {
        let exit = self.exit.lock().map_err(|e| e.to_string())?;
        if exit.stage != "idle" {
            return Err("请先完成当前关闭选择".into());
        }
        drop(exit);
        crate::desktop::minimize_to_tray(app)
    }

    pub fn request_close(self: &Arc<Self>, app: &AppHandle) {
        let mut exit = self.exit.lock().unwrap();
        if exit.stage != "idle" || self.is_stopped() {
            return;
        }
        let action = self
            .inner
            .lock()
            .unwrap()
            .monitor
            .settings
            .close_action
            .clone();
        if action == "ask" {
            *exit = ExitStatusDto {
                stage: "choose_close".into(),
                detail: String::new(),
            };
            drop(exit);
            self.notify_exit(app);
        } else {
            drop(exit);
            if action == "minimize" {
                if let Err(error) = crate::desktop::minimize_to_tray(app) {
                    *self.exit.lock().unwrap() = ExitStatusDto {
                        stage: "choose_close".into(),
                        detail: error,
                    };
                    self.notify_exit(app);
                }
            } else {
                self.begin_exit(app, false);
            }
        }
    }
    pub fn resolve_close(
        self: &Arc<Self>,
        app: &AppHandle,
        action: &str,
        remember: bool,
    ) -> Result<(), String> {
        if !matches!(action, "minimize" | "exit") {
            return Err("无效的关闭选项".into());
        }
        let _writer = self.settings_operation.lock().map_err(|e| e.to_string())?;
        if self.exit.lock().map_err(|e| e.to_string())?.stage != "choose_close" {
            return Err("关闭选择已失效".into());
        }
        if action == "minimize" {
            crate::desktop::ensure_tray(app)?;
        }
        if remember {
            self.save_settings_serialized("choose_close", |current| {
                let mut settings = current.clone();
                settings.close_action = action.into();
                Ok((settings, current.revision))
            })?;
        }
        let mut exit = self.exit.lock().map_err(|e| e.to_string())?;
        if exit.stage != "choose_close" {
            return Err("关闭选择已失效".into());
        }
        if action == "minimize" {
            // Window operations may wait for the main thread; never hold its
            // exit-state mutex while dispatching from this blocking worker.
            drop(exit);
            crate::desktop::minimize_to_tray(app)?;
            let mut exit = self.exit.lock().map_err(|e| e.to_string())?;
            if exit.stage == "choose_close" {
                *exit = ExitStatusDto::default();
            }
            drop(exit);
            let _ = app.emit("pinmeter-exit", self.exit_status());
        } else {
            *exit = ExitStatusDto::default();
            drop(exit);
            self.begin_exit(app, false);
        }
        Ok(())
    }
    pub fn exit_status(&self) -> ExitStatusDto {
        self.exit.lock().unwrap().clone()
    }
    fn notify_exit(&self, app: &AppHandle) {
        let _ = app.emit("pinmeter-exit", self.exit_status());
        if let Some(window) = app.get_webview_window("main") {
            let _ = crate::bridges::activate(&window);
        }
    }
    pub fn cancel_exit(&self, app: &AppHandle) -> Result<(), String> {
        let _writer = self
            .settings_operation
            .try_lock()
            .map_err(|_| "正在保存设置，请稍候")?;
        let mut state = self.exit.lock().unwrap();
        if matches!(state.stage.as_str(), "releasing" | "updating") {
            return Err("正在解除限制，请稍候".into());
        }
        *state = ExitStatusDto::default();
        drop(state);
        self.notify_exit(app);
        Ok(())
    }
    pub fn begin_exit(self: &Arc<Self>, app: &AppHandle, confirmed: bool) {
        let mut state = self.exit.lock().unwrap();
        if matches!(state.stage.as_str(), "releasing" | "updating") || self.is_stopped() {
            return;
        }
        state.stage = "releasing".into();
        state.detail = "正在等待已接纳的设置操作并处理退出…".into();
        drop(state);
        self.notify_exit(app);
        let runtime = self.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            // Marking releasing rejects new writers; wait off the event loop for
            // an accepted save, then read the final authoritative exit preference.
            if let Err(error) = runtime.prepare_shutdown(confirmed) {
                let stage = if matches!(error, PreparationError::Confirmation(_)) {
                    "confirm"
                } else {
                    "failed"
                };
                *runtime.exit.lock().unwrap() = ExitStatusDto {
                    stage: stage.into(),
                    detail: error.to_string(),
                };
                runtime.notify_exit(&app);
                return;
            }
            runtime.stop();
            app.exit(0);
        });
    }

    fn prepare_shutdown(&self, confirmed: bool) -> Result<(), PreparationError> {
        let _writer = self.settings_operation.lock().unwrap();
        let release = self
            .inner
            .lock()
            .unwrap()
            .monitor
            .settings
            .release_network_on_exit;
        let control = self.control.lock().unwrap().clone();
        match control {
            Some(control) if release => control.release_all(None, true).map_err(|e| {
                PreparationError::Failed(format!("未退出 Pinmeter。部分限制可能仍然生效：{e}"))
            }),
            Some(control) => match control.freeze_and_has_blocks() {
                Ok(true) if !confirmed => {
                    control.unfreeze();
                    Err(PreparationError::Confirmation("退出后，已禁用的应用仍无法联网；限速会停止。可返回设置开启退出时解除限制。".into()))
                }
                Ok(_) => Ok(()),
                Err(error) if !confirmed => {
                    control.unfreeze();
                    Err(PreparationError::Confirmation(format!(
                        "无法确认现有禁用状态：{error}。继续退出可能保留网络禁用；限速会停止。"
                    )))
                }
                Err(_) => Ok(()),
            },
            None => Ok(()),
        }
    }
    pub(crate) fn prepare_update(&self, confirmed: bool) -> Result<(), PreparationError> {
        {
            let mut state = self.exit.lock().unwrap();
            if state.stage != "idle" || self.is_stopped() {
                return Err(PreparationError::Failed("请先完成当前退出操作".into()));
            }
            state.stage = "updating".into();
        }
        if let Err(error) = self.prepare_shutdown(confirmed) {
            *self.exit.lock().unwrap() = ExitStatusDto::default();
            return Err(error);
        }
        Ok(())
    }
}
