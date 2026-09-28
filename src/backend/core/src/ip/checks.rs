//! Bounded local checks. Sampling rules adapted from one-ip src/views/link/api.ts.
//! AGPL-3.0; see platform/src/ip/third-party/NOTICE.
use super::{IpError, Query, QueryStatus};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckKind {
    Connectivity,
    Ai,
    Services,
}
impl CheckKind {
    pub const ALL: [Self; 3] = [Self::Connectivity, Self::Ai, Self::Services];
    pub fn id(self) -> &'static str {
        match self {
            Self::Connectivity => "connectivity",
            Self::Ai => "ai",
            Self::Services => "services",
        }
    }
    pub fn targets(self) -> &'static [&'static str] {
        match self {
            Self::Connectivity => &[
                "bytedance",
                "wechat",
                "taobao",
                "youtube",
                "cloudflare",
                "github",
            ],
            Self::Ai => &[
                "deepseek",
                "qwen",
                "kimi",
                "chatgpt",
                "claude",
                "perplexity",
                "grok",
                "gemini",
            ],
            Self::Services => &[
                "openai",
                "cloudflare",
                "supabase",
                "claude",
                "perplexity",
                "cursor",
                "github",
                "vercel",
            ],
        }
    }
    fn ttl(self) -> u64 {
        if self == Self::Services {
            300_000
        } else {
            60_000
        }
    }
}
#[derive(Clone, Debug)]
pub struct ServiceItem {
    pub name: String,
    pub state: String,
    pub updated_at: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ServiceReport {
    pub state: String,
    pub source: String,
    pub updated_at: Option<String>,
    pub incidents: Vec<ServiceItem>,
    pub components: Vec<ServiceItem>,
}
#[derive(Clone, Debug)]
pub struct ProbeReport {
    pub latency_ms: u32,
    pub http_status: Option<u16>,
    pub retry_after_ms: u64,
}
impl ProbeReport {
    pub fn sample(latency_ms: u32) -> Self {
        Self {
            latency_ms,
            http_status: None,
            retry_after_ms: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub enum CheckData {
    Probe(ProbeReport),
    Service(ServiceReport),
}
pub enum CheckResult {
    Probe(Result<ProbeReport, IpError>),
    Service(Result<ServiceReport, IpError>),
}
#[derive(Clone, Copy, Debug)]
pub struct CheckJob {
    pub kind: CheckKind,
    pub generation: u64,
    pub index: usize,
}
pub struct CheckRow {
    pub id: &'static str,
    pub query: Query<CheckData>,
    pub samples: Vec<Option<u32>>,
    pub note: Option<String>,
    in_flight: bool,
}
pub struct CheckGroup {
    pub kind: CheckKind,
    pub generation: u64,
    pub running: bool,
    pub retry_at: u64,
    pub rows: Vec<CheckRow>,
    deadline: u64,
    pending: VecDeque<usize>,
}
impl CheckGroup {
    pub fn new(kind: CheckKind) -> Self {
        Self {
            kind,
            generation: 0,
            running: false,
            retry_at: 0,
            deadline: 0,
            pending: VecDeque::new(),
            rows: kind
                .targets()
                .iter()
                .map(|id| CheckRow {
                    id,
                    query: Query::default(),
                    samples: Vec::new(),
                    note: None,
                    in_flight: false,
                })
                .collect(),
        }
    }
    pub fn begin(&mut self, force: bool, now: u64) -> bool {
        if self.running || now < self.retry_at {
            return false;
        }
        if !force
            && self.rows.iter().all(|r| {
                r.query.status == QueryStatus::Ready
                    && r.query
                        .valid_mono
                        .is_some_and(|t| now.saturating_sub(t) < self.kind.ttl())
            })
        {
            return false;
        }
        self.generation += 1;
        self.running = true;
        self.deadline = now.saturating_add(60_000);
        self.pending = (0..self.rows.len()).collect();
        for row in &mut self.rows {
            row.query.status = QueryStatus::Loading;
            row.query.error = None;
            row.samples.clear();
            row.note = None;
            row.in_flight = false;
        }
        true
    }
    pub fn take_job(&mut self) -> Option<CheckJob> {
        let index = self.pending.pop_front()?;
        self.rows[index].in_flight = true;
        Some(CheckJob {
            kind: self.kind,
            generation: self.generation,
            index,
        })
    }
    pub fn accept(&mut self, job: CheckJob, result: CheckResult, now: u64, wall: u64) -> bool {
        if !self.running || job.kind != self.kind || job.generation != self.generation {
            return false;
        }
        let Some(row) = self.rows.get_mut(job.index).filter(|r| r.in_flight) else {
            return false;
        };
        row.in_flight = false;
        let error = match &result {
            CheckResult::Probe(r) => r.as_ref().err(),
            CheckResult::Service(r) => r.as_ref().err(),
        };
        if let Some(e) = error {
            self.retry_at = self
                .retry_at
                .max(now.saturating_add(e.retry_after_ms.max(10_000)));
            row.note = Some(e.detail.clone());
        }
        match result {
            CheckResult::Service(result) => {
                row.query.accept(result.map(CheckData::Service), now, wall)
            }
            CheckResult::Probe(result) => {
                if let Ok(report) = &result
                    && report.retry_after_ms > 0
                {
                    self.retry_at = self.retry_at.max(now.saturating_add(report.retry_after_ms));
                }
                let stop = result.as_ref().err().is_some_and(|e| {
                    matches!(e.code, "rate_limited" | "unsupported" | "restricted")
                });
                row.samples.push(result.as_ref().ok().map(|r| r.latency_ms));
                let failures = row.samples.iter().rev().take_while(|s| s.is_none()).count();
                if self.kind == CheckKind::Connectivity
                    && row.samples.len() < 8
                    && failures < 2
                    && !stop
                {
                    self.pending.push_back(job.index);
                } else {
                    let mut successful: Vec<u32> = row.samples.iter().flatten().copied().collect();
                    successful.sort_unstable();
                    let report = if successful.is_empty() {
                        Err(result
                            .err()
                            .unwrap_or_else(|| IpError::new("connection", "未取得有效响应")))
                    } else {
                        let middle = successful.len() / 2;
                        let median = (successful[middle] as u64
                            + successful[(successful.len() - 1) / 2] as u64)
                            .div_ceil(2) as u32;
                        let mut report = result.unwrap_or_else(|_| ProbeReport::sample(median));
                        report.latency_ms = median;
                        Ok(CheckData::Probe(report))
                    };
                    row.query.accept(report, now, wall);
                }
            }
        }
        if self
            .rows
            .iter()
            .all(|r| r.query.status != QueryStatus::Loading)
        {
            self.running = false;
            self.retry_at = self.retry_at.max(now.saturating_add(10_000));
        }
        true
    }
    pub fn cancel(&mut self, invalidate: bool) {
        self.generation += 1;
        self.running = false;
        self.pending.clear();
        for row in &mut self.rows {
            row.in_flight = false;
            if invalidate || row.query.status == QueryStatus::Loading {
                row.query.stale();
            }
        }
    }
    pub fn tick(&mut self, now: u64) -> bool {
        let mut changed = false;
        if self.running && now >= self.deadline {
            for row in &mut self.rows {
                if row.query.status == QueryStatus::Loading {
                    row.query
                        .accept(Err(IpError::new("timeout", "本轮检测超时")), now, 0);
                }
            }
            self.cancel(false);
            self.retry_at = self.retry_at.max(now.saturating_add(10_000));
            changed = true;
        }
        if self.retry_at != 0 && now >= self.retry_at {
            self.retry_at = 0;
            changed = true;
        }
        for row in &mut self.rows {
            if row.query.status == QueryStatus::Ready
                && row
                    .query
                    .valid_mono
                    .is_some_and(|t| now.saturating_sub(t) >= self.kind.ttl())
            {
                row.query.stale();
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn received_ai_responses_keep_latency_and_rate_limit() {
        let mut g = CheckGroup::new(CheckKind::Ai);
        g.begin(false, 0);
        let job = g.take_job().unwrap();
        g.accept(
            job,
            CheckResult::Probe(Ok(ProbeReport {
                latency_ms: 123,
                http_status: Some(429),
                retry_after_ms: 120_000,
            })),
            1,
            1000,
        );
        assert_eq!(g.rows[0].query.status, QueryStatus::Ready);
        assert_eq!(g.rows[0].samples, vec![Some(123)]);
        assert!(
            matches!(&g.rows[0].query.data, Some(CheckData::Probe(r)) if r.http_status == Some(429))
        );
        g.cancel(false);
        assert!(!g.begin(true, 120_000));
        assert!(g.begin(true, 120_001));
        let job = g.take_job().unwrap();
        g.accept(
            job,
            CheckResult::Probe(Err(IpError::new("timeout", "timeout"))),
            120_002,
            2000,
        );
        assert_eq!(g.rows[0].query.valid_at, Some(1000));
        assert_eq!(g.rows[0].query.status, QueryStatus::Stale);
    }
    #[test]
    fn service_failure_preserves_date_and_deadline_stops_queued_work() {
        let mut g = CheckGroup::new(CheckKind::Services);
        g.begin(false, 0);
        let job = g.take_job().unwrap();
        g.accept(
            job,
            CheckResult::Service(Ok(ServiceReport {
                state: "operational".into(),
                source: "fixture".into(),
                updated_at: None,
                incidents: vec![],
                components: vec![],
            })),
            1,
            1000,
        );
        g.tick(60_000);
        assert!(!g.running);
        assert!(g.take_job().is_none());
        assert_eq!(g.rows[0].query.valid_at, Some(1000));
        assert!(g.begin(true, 70_000));
        let job = g.take_job().unwrap();
        g.accept(
            job,
            CheckResult::Service(Err(IpError::new("invalid_response", "bad"))),
            70_001,
            2000,
        );
        assert_eq!(g.rows[0].query.valid_at, Some(1000));
        assert_eq!(g.rows[0].query.status, QueryStatus::Stale);
    }
    #[test]
    fn samples_are_bounded_and_failures_do_not_renew_last_success() {
        let mut g = CheckGroup::new(CheckKind::Connectivity);
        g.begin(false, 0);
        while let Some(job) = g.take_job() {
            g.accept(
                job,
                CheckResult::Probe(Ok(ProbeReport::sample(100))),
                1,
                1000,
            );
        }
        assert!(!g.running);
        assert!(
            g.rows
                .iter()
                .all(|r| r.samples.len() == 8 && r.query.valid_at == Some(1000))
        );
        g.begin(true, 20_000);
        while let Some(job) = g.take_job() {
            g.accept(
                job,
                CheckResult::Probe(Err(IpError::new("timeout", "timeout"))),
                20_001,
                2000,
            );
        }
        assert!(g.rows.iter().all(|r| r.samples.len() == 2
            && r.query.valid_at == Some(1000)
            && r.query.status == QueryStatus::Stale));
    }
    #[test]
    fn cancel_late_reply_and_limit_do_not_bypass_cooldown() {
        let mut g = CheckGroup::new(CheckKind::Ai);
        g.begin(false, 0);
        let old = g.take_job().unwrap();
        g.cancel(false);
        assert!(!g.accept(old, CheckResult::Probe(Ok(ProbeReport::sample(5))), 1, 1));
        g.begin(false, 2);
        let job = g.take_job().unwrap();
        g.accept(
            job,
            CheckResult::Probe(Err(IpError {
                code: "rate_limited",
                detail: "limit".into(),
                retry_after_ms: 120_000,
            })),
            3,
            3,
        );
        g.cancel(false);
        assert!(!g.begin(true, 4));
        g.tick(120_003);
        assert!(g.begin(true, 120_003));
    }
}
