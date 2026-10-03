use crate::domain::{Failure, Reading, Status};
use std::collections::VecDeque;

pub const MAX_DISKS: usize = 32;
#[derive(Clone, Debug)]
pub struct DiskSample {
    pub id: String,
    pub read: Result<f64, Failure>,
    pub write: Result<f64, Failure>,
    pub idle: Result<f64, Failure>,
}
#[derive(Clone, Debug)]
pub struct DiskPoint {
    pub id: String,
    pub read: Reading,
    pub write: Reading,
    pub activity: Reading,
}
#[derive(Clone, Debug)]
pub struct DiskFrame {
    pub mono: u64,
    pub wall: u64,
    pub generation: u64,
    pub disks: Vec<DiskPoint>,
}
pub struct DiskMonitor {
    pub frames: VecDeque<DiskFrame>,
    pub status: Status,
    pub detail: String,
    generation: u64,
}
impl Default for DiskMonitor {
    fn default() -> Self {
        Self {
            frames: VecDeque::new(),
            status: Status::Warming,
            detail: "打开页面后开始磁盘采样".into(),
            generation: 0,
        }
    }
}
impl DiskMonitor {
    pub fn reset(&mut self) {
        self.generation += 1;
    }
    pub fn accept(&mut self, result: Result<Vec<DiskSample>, Failure>, mono: u64, wall: u64) {
        if self.frames.back().is_some_and(|f| mono <= f.mono) {
            return;
        }
        if self.frames.back().is_some_and(|f| mono - f.mono > 6_000) {
            self.reset();
        }
        let samples = match result {
            Ok(samples) => {
                self.status = if samples.is_empty() {
                    Status::Unsupported
                } else {
                    Status::Normal
                };
                self.detail = if samples.is_empty() {
                    "系统未提供物理磁盘计数器"
                } else if samples.len() > MAX_DISKS {
                    "仅显示前 32 块物理磁盘"
                } else {
                    "物理磁盘 · 每 2 秒采样 · 切页后停止"
                }
                .into();
                samples
            }
            Err(error) => {
                self.status = error.status;
                self.detail = error.reason.clone();
                self.frames
                    .back()
                    .map(|f| {
                        f.disks
                            .iter()
                            .map(|d| DiskSample {
                                id: d.id.clone(),
                                read: Err(error.clone()),
                                write: Err(error.clone()),
                                idle: Err(error.clone()),
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
        };
        let disks = samples
            .into_iter()
            .take(MAX_DISKS)
            .map(|s| DiskPoint {
                id: s.id,
                read: reading(s.read, false, mono, wall),
                write: reading(s.write, false, mono, wall),
                activity: reading(s.idle.map(|v| 100. - v), true, mono, wall),
            })
            .collect();
        self.frames.push_back(DiskFrame {
            mono,
            wall,
            generation: self.generation,
            disks,
        });
        while self.frames.len() > 151
            || self
                .frames
                .front()
                .is_some_and(|f| mono.saturating_sub(f.mono) > 300_000)
        {
            self.frames.pop_front();
        }
    }
}
fn reading(value: Result<f64, Failure>, percent: bool, mono: u64, wall: u64) -> Reading {
    let value = value.and_then(|v| {
        if v.is_finite() && v >= 0. && (!percent || v <= 100.) {
            Ok(v)
        } else {
            Err(Failure::new(Status::Failed, "磁盘计数器超出有效范围"))
        }
    });
    let (value, status, detail) = match value {
        Ok(v) => (Some(v), Status::Normal, String::new()),
        Err(e) => (None, e.status, e.reason),
    };
    Reading {
        value,
        status,
        detail,
        valid_at_ms: value.map(|_| wall),
        valid_mono_ms: value.map(|_| mono),
        source: "Windows PDH / PhysicalDisk",
        semantic: if percent {
            "disk.active_time"
        } else {
            "disk.bytes_per_second"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(v: f64) -> Vec<DiskSample> {
        vec![DiskSample {
            id: "0 C:".into(),
            read: Ok(v),
            write: Ok(0.),
            idle: Ok(80.),
        }]
    }
    #[test]
    fn bounds_gaps_and_invalid_are_retained() {
        let mut monitor = DiskMonitor::default();
        for i in 0..400 {
            monitor.accept(Ok(sample(0.)), i * 2_000, i * 2_000);
        }
        assert_eq!(monitor.frames.len(), 151);
        assert_eq!(
            monitor.frames.back().unwrap().disks[0].activity.value,
            Some(20.)
        );
        monitor.accept(Ok(sample(f64::NAN)), 900_000, 900_000);
        let last = monitor.frames.back().unwrap();
        assert_eq!(last.generation, 1);
        assert_eq!(last.disks[0].read.status, Status::Failed);
        monitor.accept(
            Err(Failure::new(Status::PermissionDenied, "权限不足")),
            902_000,
            902_000,
        );
        assert_eq!(monitor.frames.back().unwrap().disks[0].read.value, None);
        monitor.accept(Ok(vec![]), 904_000, 904_000);
        assert!(monitor.frames.back().unwrap().disks.is_empty());
    }
}
