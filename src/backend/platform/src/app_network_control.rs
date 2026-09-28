#[cfg(target_os = "windows")]
use pinmeter_core::app_network_control::ControlReport;
use pinmeter_core::app_network_control::{ControlAdapter, Rule, RuleBook, RuleRepository};
use std::{fs, io::Write, path::PathBuf};

pub fn validate_target(rule: &Rule) -> Result<(), String> {
    rule.validate()?;
    let path = rule.path.to_lowercase();
    let root = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_lowercase()));
    let system = std::env::var("WINDIR")
        .unwrap_or_else(|_| "C:\\Windows".into())
        .to_lowercase();
    if !cfg!(target_os = "windows") {
        return Err("此平台尚不支持应用网络控制".into());
    }
    if path.starts_with("\\\\")
        || !path.ends_with(".exe")
        || !std::path::Path::new(&rule.path).is_absolute()
        || path.contains("\\..\\")
        || path.contains("\\.\\")
        || path.starts_with(&(system + "\\"))
        || root.is_some_and(|r| path.starts_with(&(r + "\\")))
    {
        return Err("此程序不支持网络控制（系统或 Pinmeter 组件）".into());
    }
    Ok(())
}

pub struct FileRules(pub PathBuf);
impl RuleRepository for FileRules {
    fn load(&self) -> Result<RuleBook, String> {
        if !self.0.exists() {
            return Ok(RuleBook::default());
        }
        if fs::metadata(&self.0).map_err(|e| e.to_string())?.len() > 262_144 {
            return Err("网络规则文件过大，原文件已保留".into());
        }
        let mut book: RuleBook =
            serde_json::from_slice(&fs::read(&self.0).map_err(|e| e.to_string())?)
                .map_err(|e| format!("网络规则损坏，停止自动控制并保留原文件：{e}"))?;
        if book.schema_version == 1 {
            book.schema_version = 2;
        }
        book.validate()?;
        Ok(book)
    }
    fn save(&self, book: &RuleBook) -> Result<(), String> {
        book.validate()?;
        let bytes = serde_json::to_vec_pretty(book).map_err(|e| e.to_string())?;
        // Leave room for the helper request envelope and never persist an unreadable journal.
        if bytes.len() > 240_000 {
            return Err("网络规则文件过大，请先清理不再使用的规则".into());
        }
        let parent = self.0.parent().ok_or("规则目录无效")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(&self.0).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsControl {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: std::sync::mpsc::Receiver<Result<String, String>>,
}
#[cfg(target_os = "windows")]
impl WindowsControl {
    pub fn new(helper: PathBuf) -> Result<Self, String> {
        use std::{
            io::{BufRead, BufReader, Read},
            os::windows::process::CommandExt,
            process::{Command, Stdio},
            sync::mpsc,
        };
        let mut child = Command::new(helper)
            .arg(std::process::id().to_string())
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("网络控制辅助程序无法启动：{e}"))?;
        let input = child.stdin.take().ok_or("控制输入不可用")?;
        let mut output = BufReader::new(child.stdout.take().ok_or("控制输出不可用")?);
        let (tx, rx) = mpsc::sync_channel(2);
        std::thread::spawn(move || {
            loop {
                let mut line = String::new();
                let result = match Read::take(&mut output, 262_144).read_line(&mut line) {
                    Ok(n) if n > 0 && line.ends_with('\n') => Ok(line),
                    _ => Err("网络控制进程已退出或协议无效".into()),
                };
                let failed = result.is_err();
                if tx.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            input,
            output: rx,
        })
    }
    fn request(&mut self, command: &str, rules: &[Rule]) -> Result<ControlReport, String> {
        let bytes = serde_json::to_vec(&serde_json::json!({"command": command, "rules": rules}))
            .map_err(|e| e.to_string())?;
        if bytes.len() > 250_000 {
            return Err("控制请求过大".into());
        }
        self.input
            .write_all(&bytes)
            .and_then(|_| self.input.write_all(b"\n"))
            .and_then(|_| self.input.flush())
            .map_err(|e| e.to_string())?;
        let line = match self.output.recv_timeout(std::time::Duration::from_secs(8)) {
            Ok(line) => line?,
            Err(_) => {
                let _ = self.child.kill();
                return Err("网络控制响应超时，已停止限速进程；系统禁用规则保留".into());
            }
        };
        let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if value["ok"] != true {
            return Err(value["error"].as_str().unwrap_or("网络控制失败").into());
        }
        serde_json::from_value(value["report"].clone()).map_err(|e| e.to_string())
    }
}
#[cfg(target_os = "windows")]
impl ControlAdapter for WindowsControl {
    fn apply(&mut self, rules: &[Rule]) -> Result<ControlReport, String> {
        self.request("apply", rules)
    }
    fn inspect(&mut self) -> Result<ControlReport, String> {
        self.request("inspect", &[])
    }
    fn release_all(&mut self, rules: &[Rule]) -> Result<ControlReport, String> {
        self.request("release-all", rules)
    }
}
#[cfg(target_os = "windows")]
impl Drop for WindowsControl {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub fn adapter(helper: PathBuf) -> Result<Box<dyn ControlAdapter>, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(WindowsControl::new(helper)?))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = helper;
        Err("此平台尚不支持应用网络控制".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_rules_migrate_and_disabled_values_survive_reload() {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileRules(dir.path().join("rules.json"));
        fs::write(&repo.0, br#"{"schema_version":1,"revision":4,"rules":[{"id":"a","name":"a","path":"C:\\Apps\\a.exe","download":32000,"upload":null,"blocked":true}]}"#).unwrap();
        let book = repo.load().unwrap();
        assert_eq!(book.schema_version, 2);
        assert!(book.rules[0].enabled);
        repo.save(&book.disabled().unwrap()).unwrap();
        let restored = repo.load().unwrap();
        assert!(!restored.rules[0].enabled);
        assert_eq!(restored.rules[0].download, Some(32_000));
        assert!(restored.rules[0].blocked);
        assert!(restored.effective_rules()[0].empty());
    }
    #[test]
    fn rule_journal_is_atomic_and_corruption_is_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileRules(dir.path().join("rules.json"));
        repo.save(&RuleBook::default()).unwrap();
        assert_eq!(repo.load().unwrap().revision, 0);
        fs::write(&repo.0, b"broken").unwrap();
        assert!(repo.load().is_err());
        assert_eq!(fs::read(&repo.0).unwrap(), b"broken");
    }
    #[test]
    fn oversized_rules_do_not_replace_the_last_recoverable_journal() {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileRules(dir.path().join("rules.json"));
        repo.save(&RuleBook::default()).unwrap();
        let rules = (0..128)
            .map(|i| Rule {
                id: format!("{i}{}", "a".repeat(1000)),
                path: format!("C:\\{}\\{i}.exe", "b".repeat(1000)),
                ..Default::default()
            })
            .collect();
        assert!(
            repo.save(&RuleBook {
                rules,
                ..Default::default()
            })
            .is_err()
        );
        assert!(repo.load().unwrap().rules.is_empty());
    }
}
