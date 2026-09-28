use crate::shared::SystemClock;
use pinmeter_core::domain::{Failure, Observation, Status};
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

const SOURCE: &str = "LibreHardwareMonitor 0.9.6 / CPU";
const SEMANTIC: &str = "cpu.package_die_or_core_max_celsius";

fn observation(result: Result<f64, Failure>) -> Observation<f64> {
    SystemClock::default().observation(result, SOURCE, SEMANTIC)
}

/// One isolated producer. Reading the cache never changes the sample timestamp.
pub struct TemperatureCollector {
    latest: Arc<Mutex<Observation<f64>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl TemperatureCollector {
    pub fn start(helper: PathBuf) -> Self {
        let latest = Arc::new(Mutex::new(observation(Err(Failure::new(
            Status::Warming,
            "等待 CPU 温度采样",
        )))));
        let stop = Arc::new(AtomicBool::new(false));
        let cache = latest.clone();
        let shutdown = stop.clone();
        let worker = thread::spawn(move || {
            #[cfg(target_os = "windows")]
            windows_worker(helper, cache, shutdown);
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (helper, shutdown);
                *cache.lock().unwrap() = observation(Err(Failure::new(
                    Status::Unsupported,
                    "此平台尚未接入 CPU 温度传感器",
                )));
            }
        });
        Self {
            latest,
            stop,
            worker: Some(worker),
        }
    }

    pub fn latest(&self) -> Observation<f64> {
        self.latest.lock().unwrap().clone()
    }
}

impl Drop for TemperatureCollector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn decode(line: &str) -> Result<f64, Failure> {
    let invalid = || Failure::new(Status::Failed, "CPU 温度辅助程序返回无效数据");
    let packet: serde_json::Value = serde_json::from_str(line).map_err(|_| invalid())?;
    match packet["status"].as_str() {
        Some("normal") => packet["value"]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or_else(invalid),
        Some(status @ ("unsupported" | "permission_denied" | "failed")) => Err(Failure::new(
            match status {
                "unsupported" => Status::Unsupported,
                "permission_denied" => Status::PermissionDenied,
                _ => Status::Failed,
            },
            packet["detail"]
                .as_str()
                .unwrap_or("CPU 温度采集失败")
                .chars()
                .take(256)
                .collect::<String>(),
        )),
        _ => Err(invalid()),
    }
}

#[cfg(target_os = "windows")]
fn windows_worker(helper: PathBuf, cache: Arc<Mutex<Observation<f64>>>, stop: Arc<AtomicBool>) {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        sync::mpsc,
    };
    while !stop.load(Ordering::Relaxed) {
        *cache.lock().unwrap() =
            observation(Err(Failure::new(Status::Warming, "等待 CPU 温度采样")));
        let mut command = Command::new(&helper);
        command.arg(std::process::id().to_string());
        let spawned = command
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Err(error) => {
                let status = if error.kind() == std::io::ErrorKind::NotFound {
                    Status::Unsupported
                } else {
                    Status::Failed
                };
                *cache.lock().unwrap() =
                    observation(Err(Failure::new(status, "CPU 温度辅助程序缺失或无法启动")));
            }
            Ok(mut child) => {
                let mut input = child.stdin.take().unwrap();
                let mut output = BufReader::new(child.stdout.take().unwrap());
                // At most one response can wait; only one request is outstanding.
                let (tx, rx) = mpsc::sync_channel(1);
                let reader = thread::spawn(move || {
                    loop {
                        let mut line = String::new();
                        let read = Read::take(&mut output, 8192).read_line(&mut line);
                        if !matches!(read, Ok(1..)) || !line.ends_with('\n') {
                            break;
                        }
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                });
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if input
                        .write_all(b"sample\n")
                        .and_then(|_| input.flush())
                        .is_err()
                    {
                        *cache.lock().unwrap() = observation(Err(Failure::new(
                            Status::Failed,
                            "CPU 温度辅助程序已退出",
                        )));
                        break;
                    }
                    let deadline = Instant::now() + Duration::from_secs(3);
                    let reply = loop {
                        if stop.load(Ordering::Relaxed) {
                            break None;
                        }
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(line) => break Some(decode(&line)),
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                break Some(Err(Failure::new(
                                    Status::Failed,
                                    "CPU 温度辅助程序连接中断",
                                )));
                            }
                            Err(_) if Instant::now() >= deadline => {
                                break Some(Err(Failure::new(Status::Failed, "CPU 温度采集超时")));
                            }
                            Err(_) => (),
                        }
                    };
                    let Some(result) = reply else {
                        break;
                    };
                    let failed = result.is_err();
                    *cache.lock().unwrap() = observation(result);
                    if failed {
                        break;
                    }
                    thread::park_timeout(Duration::from_secs(1));
                }
                drop(input);
                drop(rx);
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
            }
        }
        if !stop.load(Ordering::Relaxed) {
            let retry = Instant::now() + Duration::from_secs(30);
            while Instant::now() < retry && !stop.load(Ordering::Relaxed) {
                thread::park_timeout(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_keeps_invalid_separate_from_zero() {
        assert_eq!(decode(r#"{"status":"normal","value":0}"#).unwrap(), 0.0);
        assert!(decode(r#"{"status":"normal","value":null}"#).is_err());
        assert!(decode("{}").is_err());
        assert_eq!(
            decode(r#"{"status":"unsupported","detail":"missing driver"}"#)
                .unwrap_err()
                .status,
            Status::Unsupported
        );
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn slow_helper_times_out_and_shutdown_joins_the_reader() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("SlowHelper.cs");
        let helper = temp.path().join("slow.exe");
        std::fs::write(&source, "class SlowHelper { static void Main() { System.Console.ReadLine(); System.Threading.Thread.Sleep(30000); } }").unwrap();
        let compiler = PathBuf::from(std::env::var_os("WINDIR").unwrap())
            .join("Microsoft.NET/Framework64/v4.0.30319/csc.exe");
        let compiled = std::process::Command::new(compiler)
            .arg("/nologo")
            .arg(format!("/out:{}", helper.display()))
            .arg(source)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stdout)
        );
        let collector = TemperatureCollector::start(helper);
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            let current = collector.latest();
            if let Err(error) = current.result
                && error.status == Status::Failed
            {
                assert!(error.reason.contains("超时"));
                break;
            }
            assert!(Instant::now() < deadline, "helper timeout was not enforced");
            thread::sleep(Duration::from_millis(20));
        }
        let at = Instant::now();
        drop(collector);
        assert!(at.elapsed() < Duration::from_secs(1));
    }
}
