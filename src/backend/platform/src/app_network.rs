use pinmeter_core::{
    app_network::{NetworkWindow, ProcessBytes},
    domain::{Failure, Status},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
};

type Update = (u64, Result<NetworkWindow, Failure>);
#[cfg(target_os = "windows")]
mod etw;
pub struct NetworkCollector {
    latest: Arc<Mutex<Option<Update>>>,
    stop: Arc<AtomicBool>,
    session: Arc<AtomicU64>,
    pub interval: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
}
impl NetworkCollector {
    pub fn start(helper: PathBuf, interval_ms: u64, generation: u64) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let interval = Arc::new(AtomicU64::new(interval_ms));
        let session = Arc::new(AtomicU64::new(generation));
        let desired = session.clone();
        let (cache, shutdown, cadence) = (latest.clone(), stop.clone(), interval.clone());
        let worker = thread::spawn(move || {
            #[cfg(target_os = "windows")]
            run(helper, cache, shutdown, cadence, desired);
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (helper, shutdown, cadence, desired);
                *cache.lock().unwrap() = Some((
                    generation,
                    Err(Failure::new(
                        Status::Unsupported,
                        "此平台尚未接入应用网络采集",
                    )),
                ));
            }
        });
        Self {
            latest,
            stop,
            session,
            interval,
            worker: Some(worker),
        }
    }
    pub fn latest(&self) -> Option<Update> {
        self.latest.lock().unwrap().clone()
    }
    pub fn set_session(&self, generation: u64) {
        if self.session.swap(generation, Ordering::Relaxed) != generation
            && let Some(worker) = &self.worker
        {
            worker.thread().unpark();
        }
    }
    pub fn is_finished(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(|worker| worker.is_finished())
    }
}
impl Drop for NetworkCollector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn decode(line: &str) -> Result<NetworkWindow, Failure> {
    let invalid = || Failure::new(Status::Failed, "应用网络采集返回无效数据");
    let value: serde_json::Value = serde_json::from_str(line).map_err(|_| invalid())?;
    if value["status"] != "normal" {
        return Err(Failure::new(
            match value["status"].as_str() {
                Some("permission_denied") => Status::PermissionDenied,
                Some("unsupported") => Status::Unsupported,
                _ => Status::Failed,
            },
            value["detail"]
                .as_str()
                .unwrap_or("应用网络采集失败")
                .chars()
                .take(256)
                .collect::<String>(),
        ));
    }
    let number = |v: &serde_json::Value| -> Result<u64, Failure> {
        v.as_str().and_then(|s| s.parse().ok()).ok_or_else(invalid)
    };
    let rows = value["rows"]
        .as_array()
        .filter(|v| v.len() <= 512)
        .ok_or_else(invalid)?;
    Ok(NetworkWindow {
        other_download: number(&value["other_download"])?,
        other_upload: number(&value["other_upload"])?,
        sequence: number(&value["sequence"])?,
        elapsed_ms: value["elapsed_ms"]
            .as_u64()
            .filter(|v| *v <= 86_400_000)
            .ok_or_else(invalid)?,
        lost: number(&value["lost"])?,
        unknown_download: number(&value["unknown_download"])?,
        unknown_upload: number(&value["unknown_upload"])?,
        rows: rows
            .iter()
            .map(|row| {
                let path = row["path"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 4096)
                    .ok_or_else(invalid)?;
                let started = number(&row["started"])?;
                Ok(ProcessBytes {
                    icon: row["icon"]
                        .as_str()
                        .filter(|s| {
                            s.len() <= 5500
                                && s.bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
                        })
                        .map(str::to_owned),
                    pid: row["pid"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(invalid)?,
                    started: started.to_string(),
                    path: path.to_owned(),
                    download: number(&row["download"])?,
                    upload: number(&row["upload"])?,
                })
            })
            .collect::<Result<Vec<_>, Failure>>()?,
    })
}

#[cfg(target_os = "windows")]
fn run(
    helper: PathBuf,
    cache: Arc<Mutex<Option<Update>>>,
    stop: Arc<AtomicBool>,
    interval: Arc<AtomicU64>,
    desired: Arc<AtomicU64>,
) {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        sync::mpsc,
        time::{Duration, Instant},
    };
    let owned = match etw::OwnedSession::new() {
        Ok(owned) => owned,
        Err(error) => {
            *cache.lock().unwrap() = Some((
                desired.load(Ordering::Relaxed),
                Err(Failure::new(Status::Failed, error)),
            ));
            return;
        }
    };
    let result = Command::new(helper)
        .arg(std::process::id().to_string())
        .args(["--session", &owned.argument()])
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match result {
        Ok(child) => child,
        Err(_) => {
            *cache.lock().unwrap() = Some((
                desired.load(Ordering::Relaxed),
                Err(Failure::new(
                    Status::Unsupported,
                    "应用网络辅助程序缺失或无法启动",
                )),
            ));
            return;
        }
    };
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        loop {
            let mut line = String::new();
            if !matches!(
                Read::take(&mut output, 2 * 1024 * 1024).read_line(&mut line),
                Ok(1..)
            ) || !line.ends_with('\n')
            {
                break;
            }
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut first = true;
    let mut active = 0;
    'collection: while !stop.load(Ordering::Relaxed) {
        let generation = desired.load(Ordering::Relaxed);
        if generation == 0 && active == 0 {
            if first {
                break;
            }
            if !matches!(child.try_wait(), Ok(None)) {
                break;
            }
            thread::park_timeout(Duration::from_millis(500));
            continue;
        }
        // Drain and stop the previous ETW session before accepting a new one.
        let pausing = active != 0 && active != generation;
        let command: &[u8] = if pausing { b"stop\n" } else { b"sample\n" };
        if input
            .write_all(command)
            .and_then(|_| input.flush())
            .is_err()
        {
            *cache.lock().unwrap() = Some((
                generation,
                Err(Failure::new(
                    Status::Failed,
                    "应用网络辅助程序已退出；请重新开始监控",
                )),
            ));
            break;
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = loop {
            if stop.load(Ordering::Relaxed)
                || (first && desired.load(Ordering::Relaxed) != generation)
            {
                break 'collection;
            }
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    break if pausing {
                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(value) if value["status"] == "stopped" => Ok(None),
                            _ => Err(Failure::new(Status::Failed, "应用网络采集未能停止")),
                        }
                    } else {
                        decode(&line).map(Some)
                    };
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(Failure::new(Status::Failed, "应用网络采集连接中断"));
                }
                Err(_) if Instant::now() >= deadline => {
                    break Err(Failure::new(Status::Failed, "应用网络采集超时"));
                }
                Err(_) => (),
            }
        };
        first = false;
        match result {
            Ok(None) => {
                active = 0;
                continue;
            }
            Ok(Some(sample)) => {
                active = generation;
                *cache.lock().unwrap() = Some((generation, Ok(sample)));
            }
            Err(error) => {
                *cache.lock().unwrap() = Some((generation, Err(error)));
                break;
            }
        }
        let deadline = Instant::now()
            + Duration::from_millis(interval.load(Ordering::Relaxed).clamp(1000, 5000));
        while Instant::now() < deadline
            && !stop.load(Ordering::Relaxed)
            && desired.load(Ordering::Relaxed) == generation
        {
            thread::park_timeout(Duration::from_millis(100));
        }
    }
    // Keep draining stdout: cancellation may leave a sample response ahead of the stop ACK.
    // Closing the input also gives legacy/error paths EOF so their finally cleanup can run.
    let _ = input.write_all(b"shutdown\n").and_then(|_| input.flush());
    drop(input);
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut acknowledged = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(line) => {
                acknowledged |= serde_json::from_str::<serde_json::Value>(&line)
                    .is_ok_and(|value| value["status"] == "shutdown_complete");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if !matches!(child.try_wait(), Ok(None)) {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => (),
        }
        if acknowledged && !matches!(child.try_wait(), Ok(None)) {
            break;
        }
    }
    drop(rx);
    if matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
    }
    let cleanup = child
        .wait()
        .map_err(|error| error.to_string())
        .and_then(|_| owned.ensure_stopped());
    if let Err(error) = cleanup {
        let detail = format!("应用网络 ETW 会话清理失败（{}）：{error}", owned.argument());
        eprintln!("{detail}");
        *cache.lock().unwrap() = Some((
            desired.load(Ordering::Relaxed),
            Err(Failure::new(Status::Failed, detail)),
        ));
    }
    let _ = reader.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    fn compile_helper(source: &str, directory: &std::path::Path, native: bool) -> PathBuf {
        let file = directory.join("Helper.cs");
        let helper = directory.join("helper.exe");
        std::fs::write(&file, source).unwrap();
        let compiler = PathBuf::from(std::env::var_os("WINDIR").unwrap())
            .join("Microsoft.NET/Framework64/v4.0.30319/csc.exe");
        let mut command = std::process::Command::new(compiler);
        command
            .args(["/nologo", "/platform:x64"])
            .arg(format!("/out:{}", helper.display()))
            .arg(file);
        if native {
            command.arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("network/NativeEtw.cs"));
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        helper
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn exit_during_sample_drains_reply_and_waits_for_slow_cleanup() {
        use std::time::{Duration, Instant};
        let temp = tempfile::tempdir().unwrap();
        let helper = compile_helper(
            r#"
using System;
using System.IO;
using System.Threading;
class Helper {
    static void Main() {
        string marker = typeof(Helper).Assembly.Location + ".state";
        if (Console.ReadLine() != "sample") return;
        File.WriteAllText(marker, "sampling");
        Thread.Sleep(300);
        Console.WriteLine("{\"status\":\"normal\"}");
        if (Console.ReadLine() != "shutdown") return;
        Thread.Sleep(1200);
        File.WriteAllText(marker, "cleaned");
        Console.WriteLine("{\"status\":\"shutdown_complete\"}");
    }
}"#,
            temp.path(),
            false,
        );
        let collector = NetworkCollector::start(helper.clone(), 1000, 1);
        let marker = helper.with_extension("exe.state");
        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        drop(collector);
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "cleaned");
        std::fs::remove_file(helper).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires an elevated Windows test process; creates real network ETW sessions"]
    fn real_etw_helper_hang_and_crash_are_recovered() {
        use std::time::{Duration, Instant};
        for crash in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let helper = compile_helper(
                &format!(
                    r#"
using System;
using System.IO;
using System.Threading;
class Helper {{
    static void Main(string[] args) {{
        Console.ReadLine();
        var trace = new NativeEtw(Guid.ParseExact(args[2], "N"));
        trace.Start();
        File.WriteAllText(typeof(Helper).Assembly.Location + ".ready", "ready");
        if ({crash}) Environment.Exit(1);
        Thread.Sleep(30000);
        GC.KeepAlive(trace);
    }}
}}"#,
                    crash = if crash { "true" } else { "false" }
                ),
                temp.path(),
                true,
            );
            let collector = NetworkCollector::start(helper.clone(), 1000, 1);
            let cache = collector.latest.clone();
            let ready = helper.with_extension("exe.ready");
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ready.exists() {
                assert!(Instant::now() < deadline, "real ETW helper could not start");
                thread::sleep(Duration::from_millis(10));
            }
            drop(collector);
            if let Some((_, Err(error))) = &*cache.lock().unwrap() {
                assert!(!error.reason.contains("ETW"), "{error:?}");
            }
            std::fs::remove_file(helper).unwrap();
        }
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn paused_helper_is_reused_and_new_sessions_restart_the_sequence() {
        use std::time::{Duration, Instant};
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ReusableHelper.cs");
        let helper = temp.path().join("reusable.exe");
        std::fs::write(&source, r#"
using System;
using System.IO;
class ReusableHelper {
    static void Main() {
        int sequence = 0;
        string command, log = typeof(ReusableHelper).Assembly.Location + ".log";
        File.AppendAllText(log, "started\n");
        while ((command = Console.ReadLine()) != null) {
            File.AppendAllText(log, command + "\n");
            if (command == "shutdown") { Console.WriteLine("{\"status\":\"shutdown_complete\"}"); break; }
            if (command == "stop") { sequence = 0; Console.WriteLine("{\"status\":\"stopped\"}"); }
            else Console.WriteLine("{\"status\":\"normal\",\"sequence\":\"" + (++sequence) + "\",\"elapsed_ms\":1000,\"lost\":\"0\",\"rows\":[],\"unknown_download\":\"0\",\"unknown_upload\":\"0\",\"other_download\":\"0\",\"other_upload\":\"0\"}");
        }
    }
}"#).unwrap();
        let compiler = PathBuf::from(std::env::var_os("WINDIR").unwrap())
            .join("Microsoft.NET/Framework64/v4.0.30319/csc.exe");
        assert!(
            std::process::Command::new(compiler)
                .arg("/nologo")
                .arg(format!("/out:{}", helper.display()))
                .arg(source)
                .output()
                .unwrap()
                .status
                .success()
        );
        let collector = NetworkCollector::start(helper.clone(), 5000, 11);
        let wait_for = |generation| {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                if let Some((id, result)) = collector.latest()
                    && id == generation
                {
                    assert_eq!(result.unwrap().sequence, 1);
                    break;
                }
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(10));
            }
        };
        wait_for(11);
        collector.set_session(0);
        let log = helper.with_extension("exe.log");
        let deadline = Instant::now() + Duration::from_secs(3);
        while !std::fs::read_to_string(&log).unwrap().ends_with("stop\n") {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        let paused = std::fs::read_to_string(&log).unwrap();
        thread::sleep(Duration::from_millis(150));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), paused);
        assert!(!collector.is_finished());
        collector.set_session(22);
        wait_for(22);
        collector.set_session(33);
        wait_for(33);
        drop(collector);
        assert_eq!(
            std::fs::read_to_string(log).unwrap(),
            "started\nsample\nstop\nsample\nstop\nsample\nshutdown\n"
        );
        std::fs::remove_file(helper).unwrap();
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn unresponsive_helper_is_reaped_after_bounded_shutdown() {
        use std::time::{Duration, Instant};
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("WaitingHelper.cs");
        let helper = temp.path().join("waiting.exe");
        std::fs::write(&source, "class WaitingHelper { static void Main() { System.Console.ReadLine(); System.Threading.Thread.Sleep(30000); } }").unwrap();
        let compiler = PathBuf::from(std::env::var_os("WINDIR").unwrap())
            .join("Microsoft.NET/Framework64/v4.0.30319/csc.exe");
        let compiled = std::process::Command::new(compiler)
            .arg("/nologo")
            .arg(format!("/out:{}", helper.display()))
            .arg(source)
            .output()
            .unwrap();
        assert!(compiled.status.success());
        let collector = NetworkCollector::start(helper.clone(), 1000, 1);
        thread::sleep(Duration::from_millis(300));
        assert!(collector.latest().is_none());
        let at = Instant::now();
        drop(collector);
        assert!(at.elapsed() < Duration::from_secs(12));
        std::fs::remove_file(helper).unwrap();
    }
    #[test]
    fn protocol_rejects_invalid_counts_and_preserves_permissions() {
        assert!(decode("{}").is_err());
        assert_eq!(
            decode(r#"{"status":"permission_denied","detail":"cancelled"}"#)
                .unwrap_err()
                .status,
            Status::PermissionDenied
        );
        let data = r#"{"status":"normal","sequence":"1","elapsed_ms":1000,"lost":"0","rows":[],"unknown_download":"9007199254740993","unknown_upload":"0","other_download":"0","other_upload":"0"}"#;
        assert_eq!(decode(data).unwrap().unknown_download, 9007199254740993);
        assert!(decode(&data.replace("9007199254740993", "-1")).is_err());
    }
}
