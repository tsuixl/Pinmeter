pub mod dto;
use crate::runtime::Runtime;
use pinmeter_core::ip::checks::{CheckJob, CheckResult};
use pinmeter_core::{ip::*, ports::Clock};
use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use tauri::async_runtime::JoinHandle;
pub fn now() -> u64 {
    pinmeter_platform::shared::SystemClock::default().monotonic_ms()
}
fn wall() -> u64 {
    pinmeter_platform::shared::SystemClock::default().wall_ms()
}
pub struct IpExecutor {
    generation: u64,
    tasks: Vec<JoinHandle<()>>,
    tx: SyncSender<(u64, IpResult)>,
    rx: Receiver<(u64, IpResult)>,
    route: Option<u64>,
    next_route: Instant,
    network: Option<String>,
    check_tasks: Vec<(CheckJob, JoinHandle<()>)>,
    check_tx: SyncSender<(CheckJob, CheckResult)>,
    check_rx: Receiver<(CheckJob, CheckResult)>,
    next_group: usize,
}
impl IpExecutor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(8);
        let (check_tx, check_rx) = mpsc::sync_channel(16);
        Self {
            generation: 0,
            tasks: Vec::new(),
            tx,
            rx,
            route: None,
            next_route: Instant::now(),
            network: None,
            check_tasks: Vec::new(),
            check_tx,
            check_rx,
            next_group: 0,
        }
    }
    pub fn poll(&mut self, runtime: &Arc<Runtime>) {
        let mut state = runtime.inner.lock().unwrap();
        let network = format!("{:?}", state.monitor.interfaces);
        let ip = &mut state.ip;
        if self.network.as_ref().is_some_and(|old| old != &network) {
            ip.invalidate();
        }
        self.network = Some(network);
        ip.tick(now());
        if ip.active() && Instant::now() >= self.next_route {
            // Local settings read only: no DNS/PAC/network work inside the monitor lock.
            let signature = pinmeter_platform::ip::route_signature();
            if self.route.is_some_and(|old| old != signature) {
                ip.invalidate();
            }
            self.route = Some(signature);
            self.next_route = Instant::now() + Duration::from_secs(5);
        }
        if self.generation != ip.generation {
            for task in self.tasks.drain(..) {
                task.abort();
            }
            self.generation = ip.generation;
        }
        while let Ok((generation, result)) = self.rx.try_recv() {
            ip.accept(generation, result, now(), wall());
        }
        while let Ok((job, result)) = self.check_rx.try_recv() {
            if let Some(group) = ip.checks.iter_mut().find(|g| g.kind == job.kind)
                && group.accept(job, result, now(), wall())
            {
                ip.revision += 1;
            }
        }
        self.check_tasks.retain(|(job, task)| {
            if !ip.active()
                || !ip
                    .checks
                    .iter()
                    .any(|g| g.kind == job.kind && g.generation == job.generation)
            {
                task.abort();
                return false;
            }
            !task.inner().is_finished()
        });
        self.tasks.retain(|task| !task.inner().is_finished());
        while self.tasks.len() < 2 && self.tasks.len() + self.check_tasks.len() < 4 {
            let Some(job) = ip.take_job() else { break };
            let tx = self.tx.clone();
            let generation = ip.generation;
            self.tasks.push(tauri::async_runtime::spawn(async move {
                let adapter = pinmeter_platform::ip::IpAdapter::new();
                let result = match job {
                    IpJob::Exit(target) => IpResult::Exit(
                        target,
                        match adapter {
                            Ok(a) => a.detect(target).await,
                            Err(e) => Err(e),
                        },
                    ),
                    IpJob::Profile(address) => {
                        let result = match adapter {
                            Ok(a) => a.lookup(&address).await,
                            Err(e) => Err(e),
                        };
                        IpResult::Profile(address, Box::new(result))
                    }
                };
                let _ = tx.try_send((generation, result));
            }));
        }
        while ip.active() && self.tasks.len() + self.check_tasks.len() < 4 {
            let mut next = None;
            for _ in 0..ip.checks.len() {
                let index = self.next_group % ip.checks.len();
                self.next_group += 1;
                if let Some(job) = ip.checks[index].take_job() {
                    next = Some(job);
                    break;
                }
            }
            let Some(job) = next else {
                break;
            };
            let tx = self.check_tx.clone();
            self.check_tasks.push((
                job,
                tauri::async_runtime::spawn(async move {
                    let result = match pinmeter_platform::ip::IpAdapter::new() {
                        Ok(adapter) => adapter.check(job.kind, job.index).await,
                        Err(e) if job.kind == checks::CheckKind::Services => {
                            CheckResult::Service(Err(e))
                        }
                        Err(e) => CheckResult::Probe(Err(e)),
                    };
                    let _ = tx.try_send((job, result));
                }),
            ));
        }
    }
}
impl Drop for IpExecutor {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
        for (_, task) in &self.check_tasks {
            task.abort();
        }
    }
}
