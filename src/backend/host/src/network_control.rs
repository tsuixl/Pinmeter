use pinmeter_core::app_network_control::{
    ControlAdapter, ControlReport, Rule, RuleBook, RuleRepository,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct NetworkRuleDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub download: Option<u32>,
    pub upload: Option<u32>,
    pub blocked: bool,
    pub enabled: bool,
    pub status: String,
    pub detail: String,
    pub inbound_blocked: bool,
    pub outbound_blocked: bool,
    pub limiting: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct NetworkControlDto {
    pub supported: bool,
    pub revision: String,
    pub version: String,
    pub available: bool,
    pub driver_available: bool,
    pub firewall_available: bool,
    pub detail: String,
    pub rules: Vec<NetworkRuleDto>,
}
struct Change {
    target: Rule,
    action: String,
    download: Option<u32>,
    upload: Option<u32>,
    expected: u64,
    reply: mpsc::SyncSender<Result<(), String>>,
}
enum Work {
    Change(Change),
    Release {
        expected: Option<u64>,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Barrier(mpsc::SyncSender<bool>),
    Stop,
}
pub struct NetworkControl {
    snapshot: Arc<Mutex<NetworkControlDto>>,
    tx: mpsc::SyncSender<Work>,
    closing: Mutex<bool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}
fn publish(
    cache: &Mutex<NetworkControlDto>,
    book: &RuleBook,
    report: &ControlReport,
    pending: Option<&str>,
) {
    let mut dto = cache.lock().unwrap();
    dto.version = (dto.version.parse::<u64>().unwrap_or(0).wrapping_add(1)).to_string();
    dto.revision = book.revision.to_string();
    dto.available = report.available;
    dto.driver_available = report.driver_available;
    dto.firewall_available = report.firewall_available;
    dto.detail = report.detail.clone();
    dto.rules = book
        .rules
        .iter()
        .map(|r| {
            let applied = report.rules.iter().find(|v| v.id == r.id);
            NetworkRuleDto {
                id: r.id.clone(),
                name: r.name.clone(),
                path: r.path.clone(),
                download: r.download,
                upload: r.upload,
                blocked: r.blocked,
                enabled: r.enabled,
                status: if pending == Some(&r.id) {
                    "applying"
                } else {
                    applied.map_or("failed", |v| {
                        if !r.enabled && v.status == "applied" {
                            if v.inbound_blocked || v.outbound_blocked || v.limiting {
                                "partial"
                            } else {
                                "disabled"
                            }
                        } else {
                            &v.status
                        }
                    })
                }
                .into(),
                detail: applied.map_or_else(|| report.detail.clone(), |v| v.detail.clone()),
                inbound_blocked: applied.is_some_and(|v| v.inbound_blocked),
                outbound_blocked: applied.is_some_and(|v| v.outbound_blocked),
                limiting: applied.is_some_and(|v| v.limiting),
            }
        })
        .collect();
}
impl NetworkControl {
    pub fn start(path: PathBuf, helper: PathBuf) -> Arc<Self> {
        Self::start_with(
            pinmeter_platform::app_network_control::FileRules(path),
            move || pinmeter_platform::app_network_control::adapter(helper.clone()),
            cfg!(target_os = "windows"),
        )
    }
    fn start_with(
        repository: impl RuleRepository + 'static,
        mut create_adapter: impl FnMut() -> Result<Box<dyn ControlAdapter>, String> + Send + 'static,
        supported: bool,
    ) -> Arc<Self> {
        let snapshot = Arc::new(Mutex::new(NetworkControlDto {
            supported,
            revision: "0".into(),
            version: "0".into(),
            available: false,
            driver_available: false,
            firewall_available: false,
            detail: "正在读取网络控制状态".into(),
            rules: vec![],
        }));
        let (tx, rx) = mpsc::sync_channel::<Work>(4);
        let cache = snapshot.clone();
        let worker = thread::spawn(move || {
            let mut book = match repository.load() {
                Ok(book) => book,
                Err(error) => {
                    cache.lock().unwrap().detail = error;
                    return;
                }
            };
            let mut adapter = create_adapter();
            let mut report = match adapter.as_mut() {
                Ok(a) => a
                    .apply(&book.effective_rules())
                    .unwrap_or_else(|e| ControlReport {
                        detail: e,
                        ..Default::default()
                    }),
                Err(e) => ControlReport {
                    detail: e.clone(),
                    ..Default::default()
                },
            };
            publish(&cache, &book, &report, None);
            loop {
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(Work::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Ok(Work::Barrier(reply)) => {
                        let _ = reply.send(
                            book.rules.iter().any(|r| r.enabled && r.blocked)
                                || report
                                    .rules
                                    .iter()
                                    .any(|r| r.inbound_blocked || r.outbound_blocked),
                        );
                    }
                    Ok(Work::Release { expected, reply }) => {
                        let result = (|| {
                            if expected.is_some_and(|revision| revision != book.revision) {
                                return Err("规则已变化，请重试".into());
                            }
                            // Only a platform with no control implementation can skip
                            // cleanup. Empty config is not evidence of an empty OS policy.
                            if book.rules.is_empty() && !supported {
                                return Ok(());
                            }
                            if adapter.is_err() {
                                adapter = create_adapter();
                            }
                            let a = adapter.as_mut().map_err(|e| e.clone())?;
                            match pinmeter_core::app_network_control::deactivate_all(
                                &mut book,
                                &repository,
                                a.as_mut(),
                            ) {
                                Ok(value) => {
                                    report = value;
                                    Ok(())
                                }
                                Err(error) => {
                                    match a.inspect() {
                                        Ok(value) => report = value,
                                        Err(inspect_error) => {
                                            adapter = Err(inspect_error);
                                            report = ControlReport {
                                                detail: error.clone(),
                                                ..Default::default()
                                            };
                                        }
                                    }
                                    Err(error)
                                }
                            }
                        })();
                        publish(&cache, &book, &report, None);
                        let _ = reply.send(result);
                    }
                    Ok(Work::Change(change)) => {
                        let result = (|| {
                            let next = book.changed(
                                change.target.clone(),
                                &change.action,
                                change.download,
                                change.upload,
                                change.expected,
                            )?;
                            // Durable desired state is the recovery journal before either OS operation.
                            repository.save(&next)?;
                            book = next;
                            publish(&cache, &book, &report, Some(&change.target.id));
                            if adapter.is_err() {
                                adapter = create_adapter();
                            }
                            let applied = adapter
                                .as_mut()
                                .map_err(|e| e.clone())
                                .and_then(|a| a.apply(&book.effective_rules()));
                            match applied {
                                Ok(value) => report = value,
                                Err(error) => {
                                    adapter = Err(error.clone());
                                    report = ControlReport {
                                        available: cfg!(target_os = "windows"),
                                        detail: error.clone(),
                                        ..Default::default()
                                    };
                                    publish(&cache, &book, &report, None);
                                    return Err(error);
                                }
                            }
                            let result = report
                                .rules
                                .iter()
                                .find(|r| r.id == change.target.id)
                                .filter(|r| r.status == "applied")
                                .map(|_| ())
                                .ok_or_else(|| {
                                    report
                                        .rules
                                        .iter()
                                        .find(|r| r.id == change.target.id)
                                        .map_or_else(|| report.detail.clone(), |r| r.detail.clone())
                                });
                            if result.is_ok() {
                                let mut cleaned = book.clone();
                                cleaned.rules.retain(|r| {
                                    !r.empty()
                                        || !report
                                            .rules
                                            .iter()
                                            .any(|a| a.id == r.id && a.status == "applied")
                                });
                                if cleaned.rules.len() != book.rules.len() {
                                    repository.save(&cleaned)?;
                                    book = cleaned;
                                }
                            }
                            publish(&cache, &book, &report, None);
                            result
                        })();
                        // No early error may leave an "applying" state behind.
                        publish(&cache, &book, &report, None);
                        let _ = change.reply.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Ok(a) = &mut adapter {
                            match a.inspect() {
                                Ok(value) => report = value,
                                Err(e) => {
                                    adapter = Err(e.clone());
                                    report = ControlReport {
                                        available: cfg!(target_os = "windows"),
                                        detail: e,
                                        ..Default::default()
                                    };
                                }
                            }
                            publish(&cache, &book, &report, None);
                        }
                    }
                }
            }
        });
        Arc::new(Self {
            snapshot,
            tx,
            closing: Mutex::new(false),
            worker: Mutex::new(Some(worker)),
        })
    }
    pub fn snapshot(&self) -> NetworkControlDto {
        self.snapshot.lock().unwrap().clone()
    }
    pub fn target(&self, id: &str) -> Option<Rule> {
        self.snapshot
            .lock()
            .unwrap()
            .rules
            .iter()
            .find(|r| r.id == id)
            .map(|r| Rule {
                id: r.id.clone(),
                name: r.name.clone(),
                path: r.path.clone(),
                download: r.download,
                upload: r.upload,
                blocked: r.blocked,
                enabled: r.enabled,
            })
    }
    pub fn change(
        &self,
        target: Rule,
        action: String,
        download: Option<u32>,
        upload: Option<u32>,
        expected: u64,
    ) -> Result<(), String> {
        let (reply, result) = mpsc::sync_channel(1);
        let closing = self.closing.lock().unwrap();
        if *closing {
            return Err("正在退出，请等待网络限制解除".into());
        }
        self.tx
            .try_send(Work::Change(Change {
                target,
                action,
                download,
                upload,
                expected,
                reply,
            }))
            .map_err(|_| "网络控制暂不可用或正在处理其他操作")?;
        drop(closing);
        result
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| "操作等待超时，请查看实际规则状态后重试")?
    }
    pub fn release_all(&self, expected: Option<u64>, exiting: bool) -> Result<(), String> {
        let mut closing = self.closing.lock().unwrap();
        if *closing {
            return Err("正在处理退出".into());
        }
        if exiting {
            *closing = true;
        }
        let (reply, result) = mpsc::sync_channel(1);
        // Queue behind accepted changes; no new mutations can race the exit barrier.
        if self.tx.send(Work::Release { expected, reply }).is_err() {
            *closing = false;
            return Err("网络规则无法读取，不能确认限制已解除".into());
        }
        drop(closing);
        // The adapter has its own bounded timeout. Do not reopen the gate before it finishes.
        let result = result
            .recv()
            .map_err(|_| "网络控制已中断，请重试".to_string())
            .and_then(|r| r);
        if exiting && result.is_err() {
            *self.closing.lock().unwrap() = false;
        }
        result
    }
    pub fn freeze_and_has_blocks(&self) -> Result<bool, String> {
        let mut closing = self.closing.lock().unwrap();
        *closing = true;
        let (reply, result) = mpsc::sync_channel(1);
        if self.tx.send(Work::Barrier(reply)).is_err() {
            *closing = false;
            return Err("无法核对退出后的网络规则".into());
        }
        drop(closing);
        match result.recv() {
            Ok(blocked) => Ok(blocked),
            Err(_) => {
                self.unfreeze();
                Err("网络控制已中断".into())
            }
        }
    }
    pub fn unfreeze(&self) {
        *self.closing.lock().unwrap() = false;
    }
    pub fn stop(&self) {
        let _ = self.tx.send(Work::Stop);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Repository(Arc<Mutex<RuleBook>>);
    impl RuleRepository for Repository {
        fn load(&self) -> Result<RuleBook, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save(&self, book: &RuleBook) -> Result<(), String> {
            *self.0.lock().unwrap() = book.clone();
            Ok(())
        }
    }
    struct RecoveryAdapter {
        calls: Arc<AtomicUsize>,
        fail_release: bool,
    }
    impl ControlAdapter for RecoveryAdapter {
        fn apply(&mut self, _: &[Rule]) -> Result<ControlReport, String> {
            Err("startup inspection failed".into())
        }
        fn inspect(&mut self) -> Result<ControlReport, String> {
            Ok(ControlReport::default())
        }
        fn release_all(&mut self, rules: &[Rule]) -> Result<ControlReport, String> {
            assert!(rules.is_empty());
            self.calls.fetch_add(1, Ordering::SeqCst);
            if std::mem::take(&mut self.fail_release) {
                return Err("system cleanup failed".into());
            }
            Ok(ControlReport {
                available: true,
                ..Default::default()
            })
        }
    }

    #[test]
    fn empty_rules_and_failed_startup_report_do_not_bypass_cleanup() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_adapter = calls.clone();
        let saved = Arc::new(Mutex::new(RuleBook::default()));
        let control = NetworkControl::start_with(
            Repository(saved.clone()),
            move || {
                Ok(Box::new(RecoveryAdapter {
                    calls: calls_for_adapter.clone(),
                    fail_release: true,
                }))
            },
            true,
        );
        assert!(control.release_all(None, true).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Failed exit must reopen the operation gate and permit verified retry.
        assert!(control.release_all(None, true).is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(saved.lock().unwrap().revision, 2);
        control.stop();
    }

    #[test]
    fn empty_rules_require_helper_recovery_before_exit() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_factory = attempts.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_adapter = calls.clone();
        let control = NetworkControl::start_with(
            Repository(Arc::new(Mutex::new(RuleBook::default()))),
            move || {
                if attempts_for_factory.fetch_add(1, Ordering::SeqCst) < 2 {
                    return Err("helper unavailable".into());
                }
                Ok(Box::new(RecoveryAdapter {
                    calls: calls_for_adapter.clone(),
                    fail_release: false,
                }))
            },
            true,
        );
        assert!(control.release_all(None, true).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(control.release_all(None, true).is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        control.stop();
    }

    #[test]
    fn unsupported_platform_without_saved_rules_can_exit() {
        let saved = Arc::new(Mutex::new(RuleBook::default()));
        let control = NetworkControl::start_with(
            Repository(saved.clone()),
            || Err("unsupported".into()),
            false,
        );
        assert!(control.release_all(None, true).is_ok());
        assert_eq!(saved.lock().unwrap().revision, 0);
        control.stop();
    }

    #[test]
    fn disabled_intent_does_not_hide_actual_residual_blocks() {
        let cache = Mutex::new(NetworkControlDto {
            supported: true,
            revision: "0".into(),
            version: "0".into(),
            available: true,
            driver_available: true,
            firewall_available: true,
            detail: String::new(),
            rules: vec![],
        });
        let book = RuleBook {
            rules: vec![Rule {
                id: "a".into(),
                name: "browser".into(),
                path: "C:\\Apps\\browser.exe".into(),
                blocked: true,
                enabled: false,
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut report = ControlReport {
            available: true,
            rules: vec![pinmeter_core::app_network_control::AppliedRule {
                id: "a".into(),
                status: "applied".into(),
                outbound_blocked: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        publish(&cache, &book, &report, None);
        assert_eq!(cache.lock().unwrap().rules[0].status, "partial");
        report.rules[0].outbound_blocked = false;
        publish(&cache, &book, &report, None);
        let state = cache.lock().unwrap();
        assert_eq!(state.rules[0].status, "disabled");
        assert!(state.rules[0].blocked); // Saved intent remains visible independently.
        assert!(!state.rules[0].outbound_blocked);
    }
}
