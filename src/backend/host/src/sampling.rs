use std::{
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const REQUEST_LEASE: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq, Eq)]
pub enum SamplingAction {
    Collect(u64),
    Reset,
    Stop,
}

struct Demand {
    visible: bool,
    expires: Option<Instant>,
    diagnostic_until: Option<Instant>,
    next: Option<Instant>,
    generation: u64,
    reset: bool,
    stopped: bool,
}
impl Demand {
    fn wanted(&self, now: Instant) -> bool {
        (self.visible && self.expires.is_some_and(|expires| expires > now))
            || self.diagnostic_until.is_some_and(|until| until > now)
    }
    fn deadline(&self, now: Instant) -> Instant {
        self.expires
            .filter(|_| self.visible)
            .into_iter()
            .chain(self.diagnostic_until)
            .filter(|at| *at > now)
            .max()
            .unwrap_or(now)
    }
    fn interrupt(&mut self) {
        if self.next.is_some() {
            self.generation += 1;
            self.reset = true;
        }
    }
}

/// One on-demand worker: idle waits have no timer; all wakeups share this lock.
pub struct SamplingWait {
    demand: Mutex<Demand>,
    changed: Condvar,
}
impl SamplingWait {
    pub fn new(visible: bool) -> Self {
        Self {
            demand: Mutex::new(Demand {
                visible,
                expires: None,
                diagnostic_until: None,
                next: None,
                generation: 0,
                reset: false,
                stopped: false,
            }),
            changed: Condvar::new(),
        }
    }
    pub fn request(&self) {
        let mut demand = self.demand.lock().unwrap();
        if demand.visible && !demand.stopped {
            let now = Instant::now();
            // A renewed lease must not bridge a pause even if the worker was busy.
            if !demand.wanted(now) {
                demand.interrupt();
            }
            demand.expires = Some(now + REQUEST_LEASE);
            self.changed.notify_one();
        }
    }
    pub fn set_visible(&self, visible: bool) {
        let mut demand = self.demand.lock().unwrap();
        if demand.visible != visible {
            demand.visible = visible;
            if !visible {
                demand.expires = None;
                if !demand.wanted(Instant::now()) {
                    demand.interrupt();
                }
            }
            self.changed.notify_one();
        }
    }
    pub fn stop(&self) {
        self.demand.lock().unwrap().stopped = true;
        self.changed.notify_one();
    }
    pub fn diagnostic_until(&self, until: Option<Instant>) {
        let mut demand = self.demand.lock().unwrap();
        let now = Instant::now();
        let was_wanted = demand.wanted(now);
        demand.diagnostic_until = until.map(|at| at.min(now + Duration::from_secs(120)));
        if was_wanted != demand.wanted(now) {
            demand.interrupt();
        }
        self.changed.notify_one();
    }
    pub fn wait(&self) -> SamplingAction {
        let mut demand = self.demand.lock().unwrap();
        loop {
            if demand.stopped {
                return SamplingAction::Stop;
            }
            if demand.reset {
                demand.reset = false;
                demand.next = None;
                return SamplingAction::Reset;
            }
            let now = Instant::now();
            if !demand.wanted(now) {
                if demand.next.take().is_some() {
                    demand.generation += 1;
                    return SamplingAction::Reset;
                }
                demand = self.changed.wait(demand).unwrap();
            } else if demand.next.is_none_or(|next| now >= next) {
                demand.next = Some(now + SAMPLE_INTERVAL);
                return SamplingAction::Collect(demand.generation);
            } else {
                let deadline = demand.next.unwrap().min(demand.deadline(now));
                demand = self.changed.wait_timeout(demand, deadline - now).unwrap().0;
            }
        }
    }
    pub fn finish(&self, generation: u64, accept: impl FnOnce()) {
        let mut demand = self.demand.lock().unwrap();
        if !demand.stopped && generation == demand.generation && demand.wanted(Instant::now()) {
            // Serialize acceptance with hide/stop so an in-flight sample cannot
            // restore an old baseline after a visibility transition.
            accept();
            demand.next = Some(Instant::now() + SAMPLE_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, mpsc},
        thread,
    };

    #[test]
    fn explicit_diagnostic_lease_survives_hide_but_expires_and_can_be_cancelled() {
        let waiter = SamplingWait::new(false);
        waiter.diagnostic_until(Some(Instant::now() + Duration::from_secs(60)));
        let SamplingAction::Collect(generation) = waiter.wait() else {
            panic!("diagnostic demand lost")
        };
        waiter.finish(generation, || ());
        waiter.set_visible(true);
        waiter.set_visible(false);
        let mut accepted = false;
        waiter.finish(generation, || accepted = true);
        assert!(accepted);
        waiter.diagnostic_until(None);
        assert_eq!(waiter.wait(), SamplingAction::Reset);
        assert!(!waiter.demand.lock().unwrap().wanted(Instant::now()));
        waiter.diagnostic_until(Some(Instant::now() - Duration::from_millis(1)));
        assert!(!waiter.demand.lock().unwrap().wanted(Instant::now()));
    }

    #[test]
    fn request_before_wait_and_stop_while_idle_are_not_lost() {
        let waiter = Arc::new(SamplingWait::new(true));
        waiter.request();
        let SamplingAction::Collect(generation) = waiter.wait() else {
            panic!("request lost")
        };
        waiter.finish(generation, || ());
        waiter.set_visible(false);
        assert_eq!(waiter.wait(), SamplingAction::Reset);
        let (sent, received) = mpsc::channel();
        let worker_waiter = waiter.clone();
        let worker = thread::spawn(move || sent.send(worker_waiter.wait()).unwrap());
        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
        waiter.stop();
        assert_eq!(
            received.recv_timeout(Duration::from_secs(1)).unwrap(),
            SamplingAction::Stop
        );
        worker.join().unwrap();
    }

    #[test]
    fn hide_and_restore_rejects_in_flight_sample_and_rebuilds_baseline() {
        let waiter = SamplingWait::new(true);
        waiter.request();
        let SamplingAction::Collect(old) = waiter.wait() else {
            panic!("request lost")
        };
        waiter.set_visible(false);
        waiter.set_visible(true);
        waiter.request();
        waiter.finish(old, || panic!("hidden sample was accepted"));
        assert_eq!(waiter.wait(), SamplingAction::Reset);
        let SamplingAction::Collect(new) = waiter.wait() else {
            panic!("restore lost")
        };
        assert_ne!(old, new);
        let mut accepted = false;
        waiter.finish(new, || accepted = true);
        assert!(accepted);
    }

    #[test]
    fn expired_lease_resets_even_when_renewed_before_worker_wakes() {
        let waiter = SamplingWait::new(true);
        waiter.request();
        let SamplingAction::Collect(old) = waiter.wait() else {
            panic!("request lost")
        };
        waiter.demand.lock().unwrap().expires = Some(Instant::now());
        waiter.request();
        assert_eq!(waiter.wait(), SamplingAction::Reset);
        waiter.finish(old, || panic!("expired sample was accepted"));
        assert!(matches!(waiter.wait(), SamplingAction::Collect(_)));
    }

    #[test]
    fn active_wait_uses_lease_deadline_and_request_does_not_accelerate_samples() {
        let waiter = SamplingWait::new(true);
        waiter.request();
        let SamplingAction::Collect(generation) = waiter.wait() else {
            panic!("request lost")
        };
        waiter.finish(generation, || ());
        let next = waiter.demand.lock().unwrap().next;
        waiter.request();
        assert_eq!(waiter.demand.lock().unwrap().next, next);
        waiter.demand.lock().unwrap().expires = Some(Instant::now() + Duration::from_millis(20));
        assert_eq!(waiter.wait(), SamplingAction::Reset);
    }
}
