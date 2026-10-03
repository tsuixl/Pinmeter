use crate::domain::{Failure, Status};
use std::collections::HashMap;
pub const MAX_PROCESSES: usize = 4096;
#[derive(Clone, Debug)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    pub times: Result<(u64, u64), Failure>,
    pub working_set: Result<u64, Failure>,
}
#[derive(Clone, Debug)]
pub struct ProcessBatch {
    pub rows: Vec<ProcessSample>,
    pub logical_cpus: u32,
    pub truncated: bool,
}
#[derive(Clone, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    pub identity: String,
    pub name: String,
    pub cpu: Option<f64>,
    pub cpu_status: Status,
    pub working_set: Option<u64>,
    pub memory_status: Status,
}
#[derive(Default)]
pub struct ProcessRanking {
    baseline: HashMap<u32, (u64, u64, u64)>,
    pub rows: Vec<ProcessRow>,
    pub unreadable: usize,
    pub truncated: bool,
    pub sampled_at: Option<u64>,
}
impl ProcessRanking {
    pub fn reset(&mut self) {
        self.baseline.clear();
        self.sampled_at = None;
        self.rows.clear();
    }
    pub fn accept(&mut self, batch: ProcessBatch, now: u64) {
        let mut next = HashMap::new();
        self.unreadable = 0;
        self.truncated = batch.truncated || batch.rows.len() > MAX_PROCESSES;
        self.rows = batch
            .rows
            .into_iter()
            .take(MAX_PROCESSES)
            .map(|sample| {
                let (cpu, cpu_status, birth) = match sample.times {
                    Ok((birth, total)) => {
                        next.insert(sample.pid, (birth, total, now));
                        let cpu = self.baseline.get(&sample.pid).and_then(
                            |&(previous_birth, old, at)| {
                                let elapsed = now.checked_sub(at)?;
                                if previous_birth != birth
                                    || elapsed == 0
                                    || elapsed > 6_000
                                    || batch.logical_cpus == 0
                                {
                                    return None;
                                }
                                let value = total.checked_sub(old)? as f64
                                    / (elapsed as f64 * 10_000. * batch.logical_cpus as f64)
                                    * 100.;
                                (value.is_finite() && (0.0..=100.).contains(&value))
                                    .then_some(value)
                            },
                        );
                        (
                            cpu,
                            if cpu.is_some() {
                                Status::Normal
                            } else {
                                Status::Warming
                            },
                            birth,
                        )
                    }
                    Err(e) => (None, e.status, 0),
                };
                let (working_set, memory_status) = match sample.working_set {
                    Ok(value) => (Some(value), Status::Normal),
                    Err(e) => (None, e.status),
                };
                if matches!(cpu_status, Status::PermissionDenied | Status::Failed)
                    || memory_status != Status::Normal
                {
                    self.unreadable += 1;
                }
                ProcessRow {
                    pid: sample.pid,
                    identity: format!("{}:{birth}", sample.pid),
                    name: sample.name,
                    cpu,
                    cpu_status,
                    working_set,
                    memory_status,
                }
            })
            .collect();
        self.baseline = next;
        self.sampled_at = Some(now);
    }
    pub fn top(&self, memory: bool) -> Vec<&ProcessRow> {
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .filter(|r| {
                if memory {
                    r.working_set.is_some()
                } else {
                    r.cpu.is_some()
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            if memory {
                b.working_set.cmp(&a.working_set)
            } else {
                b.cpu.unwrap().total_cmp(&a.cpu.unwrap())
            }
            .then_with(|| a.pid.cmp(&b.pid))
        });
        rows.truncate(10);
        rows
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn batch(birth: u64, total: u64) -> ProcessBatch {
        ProcessBatch {
            rows: (1..=15)
                .map(|pid| ProcessSample {
                    pid,
                    name: "test".into(),
                    times: Ok((birth, total)),
                    working_set: Ok(pid as u64),
                })
                .collect(),
            logical_cpus: 4,
            truncated: false,
        }
    }
    #[test]
    fn cpu_normalizes_reuse_reset_gaps_and_top_ties() {
        let mut ranking = ProcessRanking::default();
        ranking.accept(batch(10, 0), 0);
        assert!(ranking.top(false).is_empty());
        assert_eq!(ranking.top(true)[0].pid, 15);
        ranking.accept(batch(10, 20_000_000), 2_000);
        assert_eq!(ranking.top(false).len(), 10);
        assert_eq!(ranking.top(false)[0].cpu, Some(25.));
        assert_eq!(ranking.top(false)[0].pid, 1);
        ranking.accept(batch(11, 90_000_000), 4_000);
        assert!(ranking.top(false).is_empty());
        ranking.accept(batch(11, 1), 6_000);
        assert!(ranking.top(false).is_empty());
        ranking.accept(batch(11, 90_000_000), 20_000);
        assert!(ranking.top(false).is_empty());
        let mut denied = batch(11, 100_000_000);
        denied.rows[0].times = Err(Failure::new(Status::PermissionDenied, "denied"));
        denied.rows[0].working_set = Err(Failure::new(Status::PermissionDenied, "denied"));
        ranking.accept(denied, 22_000);
        assert_eq!(ranking.unreadable, 1);
        assert!(!ranking.top(false).iter().any(|r| r.pid == 1));
        ranking.accept(
            ProcessBatch {
                rows: vec![],
                logical_cpus: 4,
                truncated: false,
            },
            24_000,
        );
        assert!(ranking.baseline.is_empty());
    }
}
