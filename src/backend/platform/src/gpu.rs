use crate::shared::SystemClock;
use pinmeter_core::{
    domain::{Failure, Observation, Status},
    gpu::GpuSample,
};
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

fn observation(result: Result<Vec<GpuSample>, Failure>) -> Observation<Vec<GpuSample>> {
    SystemClock::default().observation(result, "LibreHardwareMonitor 0.9.6 / GPU", "gpu.snapshot")
}
pub struct GpuCollector {
    latest: Arc<Mutex<Observation<Vec<GpuSample>>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl GpuCollector {
    pub fn start(helper: PathBuf) -> Self {
        let latest = Arc::new(Mutex::new(observation(Err(Failure::new(
            Status::Warming,
            "等待 GPU 采样",
        )))));
        let stop = Arc::new(AtomicBool::new(false));
        let cache = latest.clone();
        let shutdown = stop.clone();
        let worker = thread::spawn(move || {
            #[cfg(target_os = "windows")]
            run(helper, cache, shutdown);
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (helper, shutdown);
                *cache.lock().unwrap() = observation(Err(Failure::new(
                    Status::Unsupported,
                    "此平台尚未接入 GPU 采集",
                )));
            }
        });
        Self {
            latest,
            stop,
            worker: Some(worker),
        }
    }
    pub fn latest(&self) -> Observation<Vec<GpuSample>> {
        self.latest.lock().unwrap().clone()
    }
    pub fn latest_if_new(&self, previous: &mut Option<u64>) -> Option<Observation<Vec<GpuSample>>> {
        let latest = self.latest.lock().unwrap();
        if previous.is_some_and(|at| latest.mono_ms <= at) {
            return None;
        }
        *previous = Some(latest.mono_ms);
        Some(latest.clone())
    }
}
impl Drop for GpuCollector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn decode(line: &str) -> Result<Vec<GpuSample>, Failure> {
    use pinmeter_core::gpu::GpuMetric;
    let invalid = || Failure::new(Status::Failed, "GPU 采集器返回无效数据");
    let packet: serde_json::Value = serde_json::from_str(line).map_err(|_| invalid())?;
    let failure = |item: &serde_json::Value| -> Result<Failure, Failure> {
        let status = match item["status"].as_str() {
            Some("warming") => Status::Warming,
            Some("unsupported") => Status::Unsupported,
            Some("permission_denied") => Status::PermissionDenied,
            Some("failed") => Status::Failed,
            _ => return Err(invalid()),
        };
        Ok(Failure::new(
            status,
            item["detail"]
                .as_str()
                .unwrap_or("GPU 采集失败")
                .chars()
                .take(256)
                .collect::<String>(),
        ))
    };
    if packet["status"] != "normal" {
        return Err(failure(&packet)?);
    }
    let devices = packet["devices"]
        .as_array()
        .filter(|d| d.len() <= 16)
        .ok_or_else(invalid)?;
    devices
        .iter()
        .map(|device| {
            let bounded = |key: &str, max| {
                device[key]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= max)
                    .map(str::to_owned)
                    .ok_or_else(invalid)
            };
            let readings = GpuMetric::ALL
                .into_iter()
                .map(|metric| {
                    let value = &device["readings"][metric.key()];
                    let reading = if value["status"] == "normal" {
                        Ok(value["value"]
                            .as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or_else(invalid)?)
                    } else {
                        Err(failure(value)?)
                    };
                    Ok((metric, reading))
                })
                .collect::<Result<_, Failure>>()?;
            Ok(GpuSample {
                id: bounded("id", 512)?,
                name: bounded("name", 256)?,
                readings,
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn run(helper: PathBuf, cache: Arc<Mutex<Observation<Vec<GpuSample>>>>, stop: Arc<AtomicBool>) {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        sync::mpsc,
    };
    while !stop.load(Ordering::Relaxed) {
        let spawned = Command::new(&helper)
            .arg(std::process::id().to_string())
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Err(error) => {
                *cache.lock().unwrap() = observation(Err(Failure::new(
                    if error.kind() == std::io::ErrorKind::NotFound {
                        Status::Unsupported
                    } else {
                        Status::Failed
                    },
                    "GPU 辅助程序缺失或无法启动",
                )));
            }
            Ok(mut child) => {
                let mut input = child.stdin.take().unwrap();
                let mut output = BufReader::new(child.stdout.take().unwrap());
                let (tx, rx) = mpsc::sync_channel(1);
                let reader = thread::spawn(move || {
                    loop {
                        let mut line = String::new();
                        if !matches!(
                            Read::take(&mut output, 131072).read_line(&mut line),
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
                while !stop.load(Ordering::Relaxed) {
                    if input
                        .write_all(b"sample\n")
                        .and_then(|_| input.flush())
                        .is_err()
                    {
                        *cache.lock().unwrap() =
                            observation(Err(Failure::new(Status::Failed, "GPU 采集器已退出")));
                        break;
                    }
                    let deadline = Instant::now() + Duration::from_secs(3);
                    let result = loop {
                        if stop.load(Ordering::Relaxed) {
                            break None;
                        }
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(line) => break Some(decode(&line)),
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                break Some(Err(Failure::new(
                                    Status::Failed,
                                    "GPU 采集器连接中断",
                                )));
                            }
                            Err(_) if Instant::now() >= deadline => {
                                break Some(Err(Failure::new(Status::Failed, "GPU 采集超时")));
                            }
                            Err(_) => (),
                        }
                    };
                    let Some(result) = result else {
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
            thread::park_timeout(Duration::from_secs(30));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_gpu_samples_are_not_delivered_again_and_failures_keep_their_time() {
        let mut initial = observation(Err(Failure::new(Status::Warming, "warming")));
        initial.mono_ms = 10;
        let collector = GpuCollector {
            latest: Arc::new(Mutex::new(initial)),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
        };
        let mut previous = None;
        assert_eq!(
            collector
                .latest_if_new(&mut previous)
                .unwrap()
                .result
                .unwrap_err()
                .status,
            Status::Warming
        );
        assert!(collector.latest_if_new(&mut previous).is_none());
        let mut failure = observation(Err(Failure::new(Status::Failed, "failed")));
        failure.mono_ms = 20;
        *collector.latest.lock().unwrap() = failure;
        let delivered = collector.latest_if_new(&mut previous).unwrap();
        assert_eq!(delivered.mono_ms, 20);
        assert_eq!(delivered.result.unwrap_err().status, Status::Failed);
        assert!(collector.latest_if_new(&mut previous).is_none());
        let mut recovered = observation(Ok(vec![]));
        recovered.mono_ms = 30;
        *collector.latest.lock().unwrap() = recovered;
        assert!(
            collector
                .latest_if_new(&mut previous)
                .unwrap()
                .result
                .is_ok()
        );
    }
    #[test]
    fn malformed_success_does_not_become_zero_or_empty_devices() {
        assert!(decode("{}").is_err());
        assert!(
            decode(r#"{"status":"normal","devices":[{"id":"x","name":"x","readings":{}}]}"#)
                .is_err()
        );
        assert!(
            decode(r#"{"status":"normal","devices":[]}"#)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            decode(r#"{"status":"permission_denied"}"#)
                .unwrap_err()
                .status,
            Status::PermissionDenied
        );
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn missing_helper_is_optional_and_stops_without_backoff_wait() {
        let collector = GpuCollector::start(PathBuf::from("Z:/pinmeter-missing-gpu-helper.exe"));
        let deadline = Instant::now() + Duration::from_secs(3);
        while collector.latest().result.as_ref().err().unwrap().status == Status::Warming {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            collector.latest().result.unwrap_err().status,
            Status::Unsupported
        );
        let at = Instant::now();
        drop(collector);
        assert!(at.elapsed() < Duration::from_secs(1));
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
        let collector = GpuCollector::start(helper);
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
