use super::*;
use std::collections::{HashSet, VecDeque};
pub const EXIT_TTL: u64 = 60_000;
pub const PROFILE_TTL: u64 = 300_000;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IpJob {
    Exit(ExitTarget),
    Profile(String),
}
pub enum IpResult {
    Exit(ExitTarget, Result<ExitAddress, IpError>),
    Profile(String, Box<Result<IpProfile, IpError>>),
}
pub struct ProfileEntry {
    pub address: String,
    pub query: Query<IpProfile>,
}
pub struct IpInspection {
    pub checks: Vec<checks::CheckGroup>,
    pub revision: u64,
    pub generation: u64,
    pub running: bool,
    pub retry_at: u64,
    pub exits: Vec<(ExitTarget, Query<ExitAddress>)>,
    pub profiles: VecDeque<ProfileEntry>,
    interests: HashSet<String>,
    pending: VecDeque<IpJob>,
    outstanding: usize,
    requested_profiles: HashSet<String>,
    deadline: u64,
    force: bool,
    failures: u32,
    profile_limit: Option<IpError>,
}
impl Default for IpInspection {
    fn default() -> Self {
        Self {
            checks: checks::CheckKind::ALL
                .into_iter()
                .map(checks::CheckGroup::new)
                .collect(),
            revision: 0,
            generation: 0,
            running: false,
            retry_at: 0,
            exits: ExitTarget::ALL
                .into_iter()
                .map(|t| (t, Query::default()))
                .collect(),
            profiles: VecDeque::new(),
            interests: HashSet::new(),
            pending: VecDeque::new(),
            outstanding: 0,
            requested_profiles: HashSet::new(),
            deadline: 0,
            force: false,
            failures: 0,
            profile_limit: None,
        }
    }
}
impl IpInspection {
    pub fn active(&self) -> bool {
        !self.interests.is_empty()
    }
    pub fn set_active(&mut self, window: &str, active: bool, now: u64) {
        if active {
            self.interests.insert(window.into());
            let _ = self.begin(false, now);
            for group in &mut self.checks {
                group.begin(false, now);
            }
            // Re-project remaining cooldown when a view returns to a cached snapshot.
            self.revision += 1;
        } else {
            self.interests.remove(window);
            if !self.active() {
                self.cancel();
                for group in &mut self.checks {
                    group.cancel(false);
                }
            }
        }
    }
    pub fn deactivate(&mut self) {
        self.interests.clear();
        self.cancel();
        for group in &mut self.checks {
            group.cancel(false);
        }
    }
    pub fn begin(&mut self, force: bool, now: u64) -> Result<(), String> {
        if !self.active() {
            return Err("请在可见的 IP 页面刷新".into());
        }
        if self.running {
            return Ok(());
        }
        if now < self.retry_at {
            return if force {
                Err("查询冷却中，请稍后重试".into())
            } else {
                Ok(())
            };
        }
        if !force
            && self.exits.iter().all(|(_, q)| {
                q.status == QueryStatus::Ready
                    && q.valid_mono
                        .is_some_and(|t| now.saturating_sub(t) < EXIT_TTL)
            })
        {
            return Ok(());
        }
        self.generation += 1;
        self.running = true;
        self.force = force;
        self.deadline = now + 30_000;
        self.pending = ExitTarget::ALL.into_iter().map(IpJob::Exit).collect();
        self.outstanding = 3;
        self.requested_profiles.clear();
        self.profile_limit = None;
        for (_, q) in &mut self.exits {
            q.status = QueryStatus::Loading;
            q.error = None;
        }
        self.revision += 1;
        Ok(())
    }
    pub fn take_job(&mut self) -> Option<IpJob> {
        self.pending.pop_front()
    }
    pub fn refresh_checks(
        &mut self,
        kind: Option<checks::CheckKind>,
        now: u64,
    ) -> Result<(), String> {
        if !self.active() {
            return Err("请在可见的 IP 页面刷新".into());
        }
        for group in &mut self.checks {
            if kind.is_none_or(|kind| kind == group.kind) {
                if kind.is_some() && now < group.retry_at {
                    return Err("检测冷却中，请稍后重试".into());
                }
                group.begin(true, now);
            }
        }
        self.revision += 1;
        Ok(())
    }
    pub fn accept(&mut self, generation: u64, result: IpResult, now: u64, wall: u64) {
        if generation != self.generation || !self.running || !self.active() {
            return;
        }
        let (error, address) = match result {
            IpResult::Exit(target, result) => {
                let result = result.and_then(|mut v| {
                    v.address = public_address(&v.address)?;
                    if (target == ExitTarget::Ipv6) != v.address.contains(':') {
                        return Err(IpError::new(
                            "address_family",
                            "检测目标返回了错误的地址类型",
                        ));
                    }
                    Ok(v)
                });
                let error = result.as_ref().err().cloned();
                let address = result.as_ref().ok().map(|v| v.address.clone());
                if let Some((_, q)) = self.exits.iter_mut().find(|(t, _)| *t == target) {
                    q.accept(result, now, wall);
                }
                (error, address)
            }
            IpResult::Profile(address, result) => {
                let result = (*result).and_then(|v| {
                    if public_address(&v.address).as_deref() == Ok(address.as_str()) {
                        Ok(v)
                    } else {
                        Err(IpError::new(
                            "address_mismatch",
                            "资料返回的地址与查询地址不一致",
                        ))
                    }
                });
                let error = result.as_ref().err().cloned();
                if let Some(entry) = self.profiles.iter_mut().find(|p| p.address == address) {
                    entry.query.accept(result, now, wall);
                }
                if let Some(limit) = error.as_ref().filter(|e| e.code == "rate_limited") {
                    self.profile_limit = Some(limit.clone());
                    self.pending.retain(|job| {
                        if let IpJob::Profile(address) = job {
                            if let Some(entry) =
                                self.profiles.iter_mut().find(|p| &p.address == address)
                            {
                                entry.query.accept(Err(limit.clone()), now, wall);
                            }
                            self.outstanding = self.outstanding.saturating_sub(1);
                            false
                        } else {
                            true
                        }
                    });
                }
                (error, None)
            }
        };
        if let Some(error) = error {
            self.failures = self.failures.saturating_add(1);
            let delay = error
                .retry_after_ms
                .max((5_000u64 << self.failures.min(5)).min(60_000));
            self.retry_at = self.retry_at.max(now.saturating_add(delay));
        }
        if let Some(address) = address {
            self.prepare_profile(address, now);
        }
        self.outstanding = self.outstanding.saturating_sub(1);
        if self.outstanding == 0 {
            self.running = false;
            self.retry_at = self.retry_at.max(now + 10_000);
            if self.exits.iter().all(|(_, q)| q.error.is_none())
                && self.profiles.iter().all(|p| p.query.error.is_none())
            {
                self.failures = 0;
            }
        }
        self.revision += 1;
    }
    fn prepare_profile(&mut self, address: String, now: u64) {
        if !self.requested_profiles.insert(address.clone()) {
            return;
        }
        if !self.profiles.iter().any(|p| p.address == address) {
            if self.profiles.len() >= 32
                && let Some(index) = self
                    .profiles
                    .iter()
                    .position(|p| !self.requested_profiles.contains(&p.address))
            {
                self.profiles.remove(index);
            }
            self.profiles.push_back(ProfileEntry {
                address: address.clone(),
                query: Query::default(),
            });
        }
        let entry = self
            .profiles
            .iter_mut()
            .find(|p| p.address == address)
            .unwrap();
        if !self.force
            && entry.query.status == QueryStatus::Ready
            && entry
                .query
                .valid_mono
                .is_some_and(|t| now.saturating_sub(t) < PROFILE_TTL)
        {
            return;
        }
        if let Some(limit) = &self.profile_limit {
            // Keep the old timestamp: a blocked request is never a fresh observation.
            entry.query.accept(Err(limit.clone()), now, 0);
            return;
        }
        entry.query.status = QueryStatus::Loading;
        entry.query.error = None;
        self.pending.push_back(IpJob::Profile(address));
        self.outstanding += 1;
    }
    pub fn tick(&mut self, now: u64) {
        for group in &mut self.checks {
            if group.tick(now) {
                self.revision += 1;
            }
        }
        if self.running && now >= self.deadline {
            self.cancel();
            self.retry_at = self.retry_at.max(now + 10_000);
            for (_, q) in &mut self.exits {
                if matches!(q.status, QueryStatus::Idle | QueryStatus::Stale) {
                    q.error = Some(IpError::new("timeout", "本轮检测超时"));
                    if q.data.is_none() {
                        q.status = QueryStatus::Failed;
                    }
                }
            }
        }
        let mut changed = false;
        if self.retry_at != 0 && now >= self.retry_at {
            self.retry_at = 0;
            changed = true;
        }
        for (_, q) in &mut self.exits {
            if q.status == QueryStatus::Ready
                && q.valid_mono
                    .is_some_and(|t| now.saturating_sub(t) >= EXIT_TTL)
            {
                q.stale();
                changed = true;
            }
        }
        for p in &mut self.profiles {
            if p.query.status == QueryStatus::Ready
                && p.query
                    .valid_mono
                    .is_some_and(|t| now.saturating_sub(t) >= PROFILE_TTL)
            {
                p.query.stale();
                changed = true;
            }
        }
        if changed {
            self.revision += 1;
        }
    }
    pub fn invalidate(&mut self) {
        self.cancel();
        for group in &mut self.checks {
            group.cancel(true);
        }
        self.profiles.clear();
        for (_, q) in &mut self.exits {
            q.stale();
        }
        self.revision += 1;
    }
    fn cancel(&mut self) {
        self.generation += 1;
        self.running = false;
        self.pending.clear();
        self.outstanding = 0;
        for (_, q) in &mut self.exits {
            if q.status == QueryStatus::Loading {
                q.stale();
            }
        }
        for p in &mut self.profiles {
            if p.query.status == QueryStatus::Loading {
                p.query.stale();
            }
        }
        self.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn exit(ip: &str) -> Result<ExitAddress, IpError> {
        Ok(ExitAddress {
            address: ip.into(),
            source: "fixture".into(),
            route: "fixture".into(),
        })
    }
    #[test]
    fn ip_deadline_does_not_cancel_other_sections_but_leaving_page_does() {
        let mut s = IpInspection::default();
        s.set_active("main", true, 0);
        s.tick(30_000);
        assert!(!s.running);
        assert!(s.checks.iter().all(|g| g.running));
        s.set_active("main", false, 30_001);
        assert!(s.checks.iter().all(|g| !g.running));
        s.set_active("main", true, 50_000);
        s.invalidate();
        assert!(s.checks.iter().all(|g| !g.running));
    }
    #[test]
    fn normalize_and_reject_private() {
        assert_eq!(
            public_address("2606:4700:4700:0000::1111").unwrap(),
            "2606:4700:4700::1111"
        );
        for ip in [
            "127.0.0.1",
            "192.168.1.1",
            "::ffff:10.0.0.1",
            "2001:db8::1",
            "foo",
        ] {
            assert!(public_address(ip).is_err());
        }
    }
    #[test]
    fn deduplicates_and_rejects_late_results() {
        let mut s = IpInspection::default();
        s.set_active("main", true, 0);
        let g = s.generation;
        s.accept(g, IpResult::Exit(ExitTarget::Ipv4, exit("1.1.1.1")), 1, 100);
        s.accept(
            g,
            IpResult::Exit(ExitTarget::Domestic, exit("1.1.1.1")),
            2,
            101,
        );
        assert_eq!(s.profiles.len(), 1);
        assert_eq!(
            s.pending
                .iter()
                .filter(|j| matches!(j, IpJob::Profile(_)))
                .count(),
            1
        );
        s.set_active("second", true, 3);
        s.set_active("main", false, 4);
        assert!(s.running);
        s.set_active("second", false, 5);
        assert!(!s.running);
        s.accept(g, IpResult::Exit(ExitTarget::Ipv4, exit("8.8.8.8")), 6, 102);
        assert_eq!(s.exits[0].1.data.as_ref().unwrap().address, "1.1.1.1");
    }
    #[test]
    fn failure_does_not_renew_old_value_and_ttl_expires() {
        let mut q = Query::default();
        q.accept(exit("1.1.1.1"), 0, 100);
        q.accept(Err(IpError::new("timeout", "timeout")), 5, 200);
        assert_eq!(q.status, QueryStatus::Stale);
        assert_eq!(q.valid_at, Some(100));
        let mut s = IpInspection::default();
        s.exits[0].1.accept(exit("1.1.1.1"), 0, 100);
        s.tick(EXIT_TTL);
        assert_eq!(s.exits[0].1.status, QueryStatus::Stale);
    }
    #[test]
    fn profile_limit_stops_queued_and_later_lookups() {
        let mut s = IpInspection::default();
        s.set_active("main", true, 0);
        let g = s.generation;
        s.pending.clear(); // Simulate the executor taking the three exit jobs.
        s.accept(g, IpResult::Exit(ExitTarget::Ipv4, exit("1.1.1.1")), 1, 100);
        assert_eq!(s.take_job(), Some(IpJob::Profile("1.1.1.1".into())));
        s.accept(
            g,
            IpResult::Exit(ExitTarget::Domestic, exit("8.8.8.8")),
            2,
            101,
        );
        s.accept(
            g,
            IpResult::Profile(
                "1.1.1.1".into(),
                Box::new(Err(IpError {
                    code: "rate_limited",
                    detail: "limit".into(),
                    retry_after_ms: 120_000,
                })),
            ),
            3,
            102,
        );
        assert!(s.pending.is_empty());
        s.accept(
            g,
            IpResult::Exit(ExitTarget::Ipv6, exit("2606:4700:4700::1111")),
            4,
            103,
        );
        assert!(s.pending.is_empty());
        assert!(!s.running);
        assert_eq!(s.retry_at, 120_003);
        assert!(
            s.profiles
                .iter()
                .all(|p| p.query.error.as_ref().unwrap().code == "rate_limited")
        );
    }
    #[test]
    fn cache_eviction_keeps_current_round_and_deadline_rejects_late_results() {
        let mut s = IpInspection::default();
        for n in 1..=32 {
            s.profiles.push_back(ProfileEntry {
                address: format!("1.1.1.{n}"),
                query: Query::default(),
            });
        }
        s.set_active("main", true, 0);
        let g = s.generation;
        s.accept(g, IpResult::Exit(ExitTarget::Ipv4, exit("1.1.1.1")), 1, 100);
        s.accept(
            g,
            IpResult::Exit(ExitTarget::Domestic, exit("8.8.8.8")),
            2,
            101,
        );
        assert_eq!(s.profiles.len(), 32);
        assert!(s.profiles.iter().any(|p| p.address == "1.1.1.1"));
        s.tick(30_000);
        assert!(!s.running);
        assert!(s.pending.is_empty());
        s.accept(
            g,
            IpResult::Exit(ExitTarget::Ipv4, exit("9.9.9.9")),
            30_001,
            102,
        );
        assert_eq!(s.exits[0].1.data.as_ref().unwrap().address, "1.1.1.1");
        s.invalidate();
        assert!(s.profiles.is_empty());
    }
    #[test]
    fn rate_limit_survives_page_reentry_and_manual_refresh() {
        let mut s = IpInspection::default();
        s.set_active("main", true, 0);
        let g = s.generation;
        s.accept(
            g,
            IpResult::Exit(
                ExitTarget::Ipv4,
                Err(IpError {
                    code: "rate_limited",
                    detail: "limit".into(),
                    retry_after_ms: 120_000,
                }),
            ),
            5,
            5,
        );
        s.deactivate();
        s.set_active("main", true, 10);
        assert!(!s.running);
        assert!(s.begin(true, 20).is_err());
        s.tick(120_005);
        assert_eq!(s.retry_at, 0);
        assert!(s.begin(true, 120_005).is_ok());
    }
}
