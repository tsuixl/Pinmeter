//! Application control rules. OS execution and persistence belong to adapters.
use serde::{Deserialize, Serialize};

pub const MIN_RATE: u32 = 16_000;
pub const MAX_RATE: u32 = 1_000_000_000;
pub const MAX_RULES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub path: String,
    pub download: Option<u32>,
    pub upload: Option<u32>,
    pub blocked: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}
fn enabled_by_default() -> bool {
    true
}
impl Default for Rule {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            path: String::new(),
            download: None,
            upload: None,
            blocked: false,
            enabled: true,
        }
    }
}
impl Rule {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 1024
            || self.path.len() > 1024
            || self.name.len() > 256
            || self.path.contains(['\0', '\n', '\r'])
            || self.path.is_empty()
        {
            return Err("无效的应用身份".into());
        }
        for rate in [self.download, self.upload].into_iter().flatten() {
            if !(MIN_RATE..=MAX_RATE).contains(&rate) {
                return Err("限速范围为 16 KB/s 至 1000 MB/s；不限制请单独选择".into());
            }
        }
        Ok(())
    }
    pub fn limited(&self) -> bool {
        self.download.is_some() || self.upload.is_some()
    }
    pub fn empty(&self) -> bool {
        !self.blocked && !self.limited()
    }
    pub fn effective(&self) -> Self {
        let mut rule = self.clone();
        if !rule.enabled {
            rule.download = None;
            rule.upload = None;
            rule.blocked = false;
        }
        rule
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleBook {
    pub schema_version: u32,
    pub revision: u64,
    pub rules: Vec<Rule>,
}
impl Default for RuleBook {
    fn default() -> Self {
        Self {
            schema_version: 2,
            revision: 0,
            rules: vec![],
        }
    }
}
impl RuleBook {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 || self.rules.len() > MAX_RULES {
            return Err("网络规则版本或数量无效".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        for rule in &self.rules {
            rule.validate()?;
            if !ids.insert(&rule.id) || !paths.insert(rule.path.to_lowercase()) {
                return Err("网络规则身份重复".into());
            }
        }
        Ok(())
    }
    /// Empty rules remain until the OS confirms removal, allowing recovery after failure.
    pub fn changed(
        &self,
        target: Rule,
        action: &str,
        down: Option<u32>,
        up: Option<u32>,
        expected: u64,
    ) -> Result<Self, String> {
        if expected != self.revision {
            return Err("规则已变化，请重新打开菜单".into());
        }
        let mut next = self.clone();
        let mut rule = self
            .rules
            .iter()
            .find(|r| r.id == target.id)
            .cloned()
            .unwrap_or(target);
        match action {
            "limits" => {
                rule.download = down;
                rule.upload = up;
            }
            "block" => {
                rule.blocked = true;
                rule.enabled = true;
            }
            "enable" => rule.enabled = true,
            "disable" => rule.enabled = false,
            "restore" => rule.blocked = false,
            "clear" => {
                rule.download = None;
                rule.upload = None;
                rule.blocked = false;
            }
            _ => return Err("未知网络控制操作".into()),
        }
        rule.validate()?;
        next.rules.retain(|r| r.id != rule.id);
        next.rules.push(rule);
        next.revision = self.revision.checked_add(1).ok_or("规则版本已耗尽")?;
        next.validate()?;
        Ok(next)
    }
    pub fn effective_rules(&self) -> Vec<Rule> {
        self.rules.iter().map(Rule::effective).collect()
    }
    pub fn disabled(&self) -> Result<Self, String> {
        let mut next = self.clone();
        for rule in &mut next.rules {
            rule.enabled = false;
        }
        next.revision = next.revision.checked_add(1).ok_or("规则版本已耗尽")?;
        Ok(next)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppliedRule {
    pub id: String,
    pub status: String,
    pub detail: String,
    pub inbound_blocked: bool,
    pub outbound_blocked: bool,
    pub limiting: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ControlReport {
    pub available: bool,
    pub detail: String,
    pub driver_available: bool,
    pub firewall_available: bool,
    pub rules: Vec<AppliedRule>,
}
pub trait ControlAdapter: Send {
    fn apply(&mut self, rules: &[Rule]) -> Result<ControlReport, String>;
    fn inspect(&mut self) -> Result<ControlReport, String>;
    fn release_all(&mut self, rules: &[Rule]) -> Result<ControlReport, String>;
}
pub trait RuleRepository: Send {
    fn load(&self) -> Result<RuleBook, String>;
    fn save(&self, book: &RuleBook) -> Result<(), String>;
}

/// Persist inactive intent first, so a failed cleanup can be retried after restart.
pub fn deactivate_all(
    book: &mut RuleBook,
    repository: &dyn RuleRepository,
    adapter: &mut dyn ControlAdapter,
) -> Result<ControlReport, String> {
    let next = book.disabled()?;
    repository.save(&next)?;
    *book = next;
    let report = adapter.release_all(&book.effective_rules())?;
    if !report.available {
        return Err(if report.detail.is_empty() {
            "网络控制不可用，不能确认限制已解除".into()
        } else {
            format!("不能确认限制已解除：{}", report.detail)
        });
    }
    let failures: Vec<_> = book
        .rules
        .iter()
        .filter(|rule| {
            !report.rules.iter().any(|applied| {
                applied.id == rule.id
                    && applied.status == "applied"
                    && !applied.inbound_blocked
                    && !applied.outbound_blocked
                    && !applied.limiting
            })
        })
        .map(|rule| rule.name.clone())
        .collect();
    if !failures.is_empty() {
        return Err(format!(
            "未能确认解除以下应用的限制：{}",
            failures.join("、")
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> Rule {
        Rule {
            id: "a".into(),
            path: "C:\\Apps\\a.exe".into(),
            name: "a".into(),
            ..Default::default()
        }
    }
    #[test]
    fn block_preserves_limits_and_restore_reuses_them() {
        let book = RuleBook::default()
            .changed(target(), "limits", Some(200_000), Some(16_000), 0)
            .unwrap();
        let blocked = book.changed(target(), "block", None, None, 1).unwrap();
        assert!(blocked.rules[0].blocked);
        assert_eq!(blocked.rules[0].download, Some(200_000));
        let restored = blocked.changed(target(), "restore", None, None, 2).unwrap();
        assert!(!restored.rules[0].blocked);
        assert_eq!(restored.rules[0].upload, Some(16_000));
        assert!(restored.changed(target(), "clear", None, None, 2).is_err());
        assert!(
            restored
                .changed(target(), "clear", None, None, 3)
                .unwrap()
                .rules[0]
                .empty()
        );
    }
    #[test]
    fn rejects_ambiguous_rates_and_duplicate_paths() {
        for rate in [0, 15_999, MAX_RATE + 1] {
            assert!(
                RuleBook::default()
                    .changed(target(), "limits", Some(rate), None, 0)
                    .is_err()
            );
        }
        let book = RuleBook::default()
            .changed(target(), "block", None, None, 0)
            .unwrap();
        let mut duplicate = target();
        duplicate.id = "other".into();
        assert!(book.changed(duplicate, "block", None, None, 1).is_err());
    }
    #[test]
    fn inactive_rules_preserve_values_until_explicit_enable() {
        let book = RuleBook::default()
            .changed(target(), "limits", Some(20_000), Some(30_000), 0)
            .unwrap()
            .changed(target(), "block", None, None, 1)
            .unwrap()
            .disabled()
            .unwrap();
        assert!(!book.rules[0].enabled);
        assert!(book.rules[0].blocked);
        assert_eq!(book.rules[0].download, Some(20_000));
        assert!(book.effective_rules()[0].empty());
        let edited = book
            .changed(target(), "limits", Some(40_000), None, book.revision)
            .unwrap();
        assert!(!edited.rules[0].enabled);
        assert!(edited.effective_rules()[0].empty());
        let enabled = edited
            .changed(target(), "enable", None, None, edited.revision)
            .unwrap();
        assert!(enabled.effective_rules()[0].blocked);
        assert_eq!(enabled.rules[0].download, Some(40_000));
    }
    struct Repository {
        fail: bool,
        saved: std::sync::Mutex<Option<RuleBook>>,
    }
    impl RuleRepository for Repository {
        fn load(&self) -> Result<RuleBook, String> {
            Ok(self.saved.lock().unwrap().clone().unwrap_or_default())
        }
        fn save(&self, book: &RuleBook) -> Result<(), String> {
            if self.fail {
                return Err("disk full".into());
            }
            *self.saved.lock().unwrap() = Some(book.clone());
            Ok(())
        }
    }
    struct Adapter {
        calls: usize,
        fail: bool,
        residue: bool,
        unavailable: bool,
    }
    impl ControlAdapter for Adapter {
        fn apply(&mut self, _: &[Rule]) -> Result<ControlReport, String> {
            unreachable!()
        }
        fn inspect(&mut self) -> Result<ControlReport, String> {
            unreachable!()
        }
        fn release_all(&mut self, rules: &[Rule]) -> Result<ControlReport, String> {
            self.calls += 1;
            assert!(rules.iter().all(Rule::empty));
            if self.fail {
                return Err("access denied".into());
            }
            Ok(ControlReport {
                available: !self.unavailable,
                rules: rules
                    .iter()
                    .map(|r| AppliedRule {
                        id: r.id.clone(),
                        status: "applied".into(),
                        outbound_blocked: self.residue,
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            })
        }
    }
    #[test]
    fn exit_does_not_touch_system_when_journal_save_fails() {
        let mut book = RuleBook::default()
            .changed(target(), "block", None, None, 0)
            .unwrap();
        let repo = Repository {
            fail: true,
            saved: Default::default(),
        };
        let mut adapter = Adapter {
            calls: 0,
            fail: false,
            residue: false,
            unavailable: false,
        };
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_err());
        assert!(book.rules[0].enabled);
        assert_eq!(adapter.calls, 0);
    }
    #[test]
    fn failed_exit_keeps_inactive_recovery_journal_and_can_retry() {
        let mut book = RuleBook::default()
            .changed(target(), "block", None, None, 0)
            .unwrap();
        let repo = Repository {
            fail: false,
            saved: Default::default(),
        };
        let mut adapter = Adapter {
            calls: 0,
            fail: true,
            residue: false,
            unavailable: false,
        };
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_err());
        assert!(!repo.load().unwrap().rules[0].enabled);
        assert!(book.rules[0].blocked);
        adapter.fail = false;
        adapter.residue = true;
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_err());
        adapter.residue = false;
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_ok());
        assert_eq!(repo.load().unwrap().revision, book.revision);
    }
    #[test]
    fn empty_journal_still_requires_available_cleanup_confirmation() {
        let mut book = RuleBook::default();
        let repo = Repository {
            fail: false,
            saved: Default::default(),
        };
        let mut adapter = Adapter {
            calls: 0,
            fail: false,
            residue: false,
            unavailable: true,
        };
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_err());
        assert_eq!(adapter.calls, 1);
        adapter.fail = true;
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_err());
        adapter.fail = false;
        adapter.unavailable = false;
        assert!(deactivate_all(&mut book, &repo, &mut adapter).is_ok());
        assert_eq!(adapter.calls, 3);
        assert!(repo.load().unwrap().rules.is_empty());
    }
}
