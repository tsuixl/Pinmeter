use crate::runtime::Runtime;
use pinmeter_core::domain::Settings;

impl Runtime {
    /// Lock order: settings writer -> short exit check -> short monitor access.
    /// Slow platform operations never hold either the exit or monitor mutex.
    pub fn save_settings(
        &self,
        allowed_stage: &str,
        change: impl FnOnce(&Settings) -> Result<(Settings, u64), String>,
    ) -> Result<Settings, String> {
        let _writer = self.settings_operation.lock().map_err(|e| e.to_string())?;
        self.save_settings_serialized(allowed_stage, change)
    }

    /// Caller owns settings_operation until the outcome has been applied.
    pub fn save_settings_serialized(
        &self,
        allowed_stage: &str,
        change: impl FnOnce(&Settings) -> Result<(Settings, u64), String>,
    ) -> Result<Settings, String> {
        if self.exit.lock().map_err(|e| e.to_string())?.stage != allowed_stage {
            return Err("正在处理退出或关闭选择，请返回后修改设置".into());
        }
        let prepared = {
            let state = self.inner.lock().map_err(|e| e.to_string())?;
            let (next, expected) = change(&state.monitor.settings)?;
            state.monitor.prepare_settings(next, expected)?
        };
        let outcome = prepared.execute(self.repository.as_ref(), &self.autostart);
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        state.monitor.finish_settings(outcome)?;
        Ok(state.monitor.settings.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinmeter_core::{
        domain::{Observation, Status},
        ports::SettingsRepository,
    };
    use std::{
        sync::{Arc, Mutex, mpsc},
        thread,
        time::Duration,
    };

    struct SlowRepository {
        entered: mpsc::SyncSender<()>,
        release: Mutex<mpsc::Receiver<()>>,
        fail: bool,
    }
    impl SettingsRepository for SlowRepository {
        fn load(&self) -> Result<Option<Settings>, String> {
            Ok(None)
        }
        fn save(&self, _: &Settings) -> Result<(), String> {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .map_err(|e| e.to_string())?;
            if self.fail {
                Err("disk failed".into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn slow_save_keeps_live_state_available_and_preserves_samples_on_success_or_failure() {
        for fail in [false, true] {
            let (entered, waiting) = mpsc::sync_channel(1);
            let (release, receiver) = mpsc::sync_channel(1);
            let runtime = Runtime::new(Arc::new(SlowRepository {
                entered,
                release: Mutex::new(receiver),
                fail,
            }));
            let writer = runtime.clone();
            let save = thread::spawn(move || {
                writer.save_settings("idle", |current| {
                    let mut next = current.clone();
                    next.theme = "dark".into();
                    Ok((next, current.revision))
                })
            });
            waiting.recv_timeout(Duration::from_secs(5)).unwrap();
            let available = match runtime.inner.try_lock() {
                Ok(mut state) => {
                    assert_eq!(state.monitor.settings.revision, 0);
                    state.monitor.accept_temperature(Observation {
                        result: Ok(47.0),
                        mono_ms: 100,
                        wall_ms: 100,
                        source: "test",
                        semantic: "test",
                    });
                    true
                }
                Err(_) => false,
            };
            let exit_available = runtime.exit.try_lock().is_ok();
            release.send(()).unwrap();
            let saved = save.join().unwrap();
            assert!(
                available && exit_available,
                "slow I/O held a runtime state lock"
            );
            assert_eq!(saved.is_err(), fail);
            let state = runtime.inner.lock().unwrap();
            assert_eq!(state.monitor.settings.revision, if fail { 0 } else { 1 });
            assert_eq!(state.monitor.cpu_temperature_at(100).value, Some(47.0));
            assert_eq!(state.monitor.cpu_temperature_at(100).status, Status::Normal);
        }
    }

    #[test]
    fn queued_old_revision_and_new_writes_during_exit_have_no_side_effects() {
        let (entered, waiting) = mpsc::sync_channel(1);
        let (release, receiver) = mpsc::sync_channel(1);
        let runtime = Runtime::new(Arc::new(SlowRepository {
            entered,
            release: Mutex::new(receiver),
            fail: false,
        }));
        let writer = runtime.clone();
        let first = thread::spawn(move || {
            writer.save_settings("idle", |current| {
                let mut next = current.clone();
                next.theme = "dark".into();
                Ok((next, 0))
            })
        });
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let writer = runtime.clone();
        let second =
            thread::spawn(move || writer.save_settings("idle", |_| Ok((Settings::default(), 0))));
        release.send(()).unwrap();
        assert_eq!(first.join().unwrap().unwrap().revision, 1);
        assert!(second.join().unwrap().unwrap_err().contains("版本冲突"));
        runtime.exit.lock().unwrap().stage = "releasing".into();
        assert!(
            runtime
                .save_settings("idle", |_| panic!("exit must reject before preparing"))
                .is_err()
        );
        assert!(
            waiting.try_recv().is_err(),
            "rejected write touched storage"
        );
    }
}
