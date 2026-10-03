use crate::domain::{Frame, Status};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct TraceMetric {
    pub value: Option<f64>,
    pub status: Status,
}
#[derive(Clone, Serialize)]
pub struct TracePoint {
    pub at_ms: u64,
    pub cpu: TraceMetric,
    pub memory: TraceMetric,
    pub download: TraceMetric,
    pub upload: TraceMetric,
}
impl From<&Frame> for TracePoint {
    fn from(frame: &Frame) -> Self {
        let metric = |r: &crate::domain::Reading| TraceMetric {
            value: (r.status == Status::Normal).then_some(r.value).flatten(),
            status: r.status,
        };
        Self {
            at_ms: frame.at_ms,
            cpu: metric(&frame.cpu),
            memory: metric(&frame.memory),
            download: metric(&frame.download),
            upload: metric(&frame.upload),
        }
    }
}
#[derive(Clone, Serialize)]
pub struct TraceProcess {
    pub pid: u32,
    pub name: String,
    pub cpu: f64,
}
#[derive(Clone, Serialize)]
pub struct TraceObservation {
    pub observed_at_ms: u64,
    pub metrics: Option<TracePoint>,
    pub processes_at_ms: Option<u64>,
    pub processes_status: String,
    pub top_cpu_processes: Vec<TraceProcess>,
}
#[derive(Clone, Serialize)]
pub struct TraceReport {
    pub schema_version: u32,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub duration_seconds: u32,
    pub before: Vec<TracePoint>,
    pub observations: Vec<TraceObservation>,
}
impl TraceReport {
    pub fn new(
        started_at_ms: u64,
        duration_seconds: u32,
        before: Vec<TracePoint>,
    ) -> Result<Self, String> {
        if !matches!(duration_seconds, 30 | 60 | 120) {
            return Err("排障时长只能为 30、60 或 120 秒".into());
        }
        Ok(Self {
            schema_version: 1,
            started_at_ms,
            finished_at_ms: None,
            duration_seconds,
            before: before.into_iter().take(12).collect(),
            observations: Vec::new(),
        })
    }
    pub fn record(&mut self, mut sample: TraceObservation) {
        if self.finished_at_ms.is_none() && self.observations.len() < 61 {
            sample
                .top_cpu_processes
                .retain(|p| p.cpu.is_finite() && p.cpu >= 0.);
            sample
                .top_cpu_processes
                .sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            sample.top_cpu_processes.truncate(3);
            for process in &mut sample.top_cpu_processes {
                process.name = process.name.chars().take(256).collect();
            }
            self.observations.push(sample);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trace_is_explicit_bounded_and_retains_missing_samples() {
        assert!(TraceReport::new(1, 3600, vec![]).is_err());
        let mut trace = TraceReport::new(1, 30, vec![]).unwrap();
        for _ in 0..100 {
            trace.record(TraceObservation {
                observed_at_ms: 2,
                metrics: None,
                processes_at_ms: None,
                processes_status: "warming".into(),
                top_cpu_processes: vec![TraceProcess {
                    pid: 1,
                    name: "unknown".into(),
                    cpu: f64::NAN,
                }],
            });
        }
        assert_eq!(trace.observations.len(), 61);
        assert!(trace.observations[0].metrics.is_none());
        assert!(trace.observations[0].top_cpu_processes.is_empty());
    }
}
