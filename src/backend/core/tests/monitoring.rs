use pinmeter_core::{application::Monitor, domain::*, ports::SettingsRepository};
fn observation<T>(result: Result<T, Failure>, at: u64) -> Observation<T> {
    Observation {
        result,
        mono_ms: at,
        wall_ms: at + 10000,
        source: "test",
        semantic: "test",
    }
}
#[test]
fn temperature_cache_cannot_refresh_old_or_invalid_readings() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept_temperature(observation(Ok(65.5), 1000));
    assert_eq!(monitor.cpu_temperature_at(1000).value, Some(65.5));
    monitor.accept_temperature(observation(Ok(65.5), 1000));
    assert_eq!(monitor.cpu_temperature_at(4001).status, Status::Stale);
    monitor.accept_temperature(observation(Ok(0.0), 5000));
    assert_eq!(monitor.cpu_temperature_at(5000).value, Some(0.0));
    monitor.accept_temperature(observation(Ok(99.0), 4000));
    assert_eq!(monitor.cpu_temperature_at(5000).value, Some(0.0));
    monitor.accept_temperature(observation(Ok(f64::NAN), 6000));
    assert_eq!(monitor.cpu_temperature_at(6000).status, Status::Failed);
    monitor.accept_temperature(observation(Ok(200.0), 6001));
    assert!(monitor.cpu_temperature_at(6001).value.is_none());
    monitor.accept_temperature(observation(
        Err(Failure::new(Status::Unsupported, "missing driver")),
        7000,
    ));
    assert_eq!(
        monitor.cpu_temperature_at(100000).status,
        Status::Unsupported
    );
    monitor.accept_temperature(observation(
        Err(Failure::new(Status::Warming, "waiting for authorization")),
        200000,
    ));
    assert_eq!(monitor.cpu_temperature_at(200001).status, Status::Warming);
    assert_eq!(monitor.cpu_temperature_at(320001).status, Status::Stale);
    assert!(monitor.history.is_empty());
}
fn counters(id: &str, bytes: u64) -> NetworkCounters {
    NetworkCounters {
        interface: NetworkInterface {
            id: id.into(),
            name: id.into(),
            up: true,
            physical: true,
        },
        received: bytes,
        transmitted: bytes,
    }
}
#[test]
fn temperature_history_freezes_validity_and_keeps_gaps_bounded() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept_temperature(observation(Ok(62.5), 1000));
    monitor.accept(sample(1000, 100), 0);
    monitor.accept(sample(5000, 500), 0);
    assert_eq!(monitor.history[0].cpu_temperature.value, Some(62.5));
    assert_eq!(monitor.history[0].cpu_temperature.valid_mono_ms, Some(1000));
    assert_eq!(monitor.history[1].cpu_temperature.status, Status::Stale);
    assert!(monitor.history[1].cpu_temperature.value.is_none());
    monitor.accept_temperature(observation(Ok(0.0), 6000));
    monitor.accept(sample(6000, 600), 0);
    assert_eq!(monitor.history[2].cpu_temperature.value, Some(0.0));
    assert_eq!(monitor.history[1].cpu_temperature.status, Status::Stale);
    assert_eq!(
        monitor.latest_at(10000).unwrap().cpu_temperature.status,
        Status::Stale
    );
    for second in 7..=400 {
        monitor.accept(sample(second * 1000, second * 100), 0);
    }
    assert_eq!(monitor.history.len(), 301);
    assert_eq!(monitor.history.front().unwrap().elapsed_ms, 100000);
}
fn sample(at: u64, bytes: u64) -> RawSample {
    RawSample {
        cpu: observation(Ok(10.0), at),
        cpu_processors: observation(
            Ok(vec![ProcessorSample {
                id: "0,0".into(),
                usage: Ok(20.0),
            }]),
            at,
        ),
        memory: observation(Ok(Memory { used: 4, total: 8 }), at),
        network: observation(Ok(vec![counters("a", bytes)]), at),
    }
}

#[test]
fn gpu_history_freezes_samples_and_marks_a_stopped_producer_stale() {
    use pinmeter_core::gpu::{GpuMetric, GpuSample};
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.gpu.accept(observation(
        Ok(vec![GpuSample {
            id: "gpu-a".into(),
            name: "GPU".into(),
            readings: [(GpuMetric::Usage, Ok(0.0))].into(),
        }]),
        1000,
    ));
    monitor.accept(sample(1000, 0), 0);
    monitor.accept(sample(5000, 0), 0);
    assert_eq!(
        monitor.history[0].gpus[0].readings[&GpuMetric::Usage].value,
        Some(0.0)
    );
    assert_eq!(
        monitor.history[1].gpus[0].readings[&GpuMetric::Usage].status,
        Status::Stale
    );
    assert_eq!(
        monitor.latest_at(6000).unwrap().gpus[0].readings[&GpuMetric::Usage].value,
        None
    );
}
#[test]
fn processor_failures_are_independent_and_expire_without_rewriting_history() {
    let mut monitor = Monitor::new(Settings::default(), None);
    let mut first = sample(1000, 100);
    first.cpu_processors.result = Ok(vec![
        ProcessorSample {
            id: "0,0".into(),
            usage: Ok(0.0),
        },
        ProcessorSample {
            id: "0,1".into(),
            usage: Ok(f64::NAN),
        },
        ProcessorSample {
            id: "0,2".into(),
            usage: Err(Failure::new(Status::Warming, "warming")),
        },
    ]);
    assert!(monitor.accept(first, 0));
    let current = monitor.cpu_processors_at(1000);
    assert_eq!(current.processors[0].usage.value, Some(0.0));
    assert_eq!(current.processors[1].usage.status, Status::Failed);
    assert_eq!(current.processors[1].usage.value, None);
    assert_eq!(current.processors[2].usage.status, Status::Warming);
    assert_eq!(monitor.history.back().unwrap().cpu.status, Status::Normal);
    let expired = monitor.cpu_processors_at(4001);
    assert_eq!(expired.status, Status::Stale);
    assert!(
        expired
            .processors
            .iter()
            .all(|p| p.usage.status == Status::Stale && p.usage.value.is_none())
    );
    assert_eq!(monitor.history.back().unwrap().cpu.status, Status::Normal);
}

#[test]
fn processor_batch_failure_keeps_identity_and_recovery_replaces_topology() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept(sample(1000, 100), 0);
    let mut failed = sample(2000, 200);
    failed.cpu_processors.result = Err(Failure::new(Status::Failed, "counter unavailable"));
    monitor.accept(failed, 0);
    let current = monitor.cpu_processors_at(2000);
    assert_eq!(current.status, Status::Failed);
    assert_eq!(current.processors[0].id, "0,0");
    assert_eq!(current.processors[0].usage.valid_mono_ms, Some(1000));
    assert_eq!(current.processors[0].usage.value, None);
    assert_eq!(
        monitor.cpu_processors_at(4001).processors[0].usage.status,
        Status::Stale
    );
    let mut recovered = sample(5000, 500);
    recovered.cpu_processors.result = Ok(vec![ProcessorSample {
        id: "1,0".into(),
        usage: Ok(33.0),
    }]);
    assert!(!monitor.accept(recovered.clone(), 1));
    assert_eq!(monitor.cpu_processors_at(5000).processors[0].id, "0,0");
    assert!(monitor.accept(recovered, 0));
    let current = monitor.cpu_processors_at(5000);
    assert_eq!(current.status, Status::Normal);
    assert_eq!(current.processors.len(), 1);
    assert_eq!(current.processors[0].id, "1,0");
    assert_eq!(current.processors[0].usage.value, Some(33.0));
}

#[test]
fn invalid_processor_inventory_does_not_replace_known_ids_with_ambiguous_rows() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept(sample(1000, 100), 0);
    for (index, rows) in [
        vec![],
        vec![
            ProcessorSample {
                id: "0,1".into(),
                usage: Ok(100.0),
            },
            ProcessorSample {
                id: "0,1".into(),
                usage: Ok(10.0),
            },
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let at = 2000 + index as u64 * 1000;
        let mut invalid = sample(at, 200);
        invalid.cpu_processors.result = Ok(rows);
        monitor.accept(invalid, 0);
        let current = monitor.cpu_processors_at(at);
        assert_eq!(current.status, Status::Failed);
        assert_eq!(current.processors.len(), 1);
        assert_eq!(current.processors[0].id, "0,0");
        assert_eq!(current.processors[0].usage.value, None);
    }
}

#[test]
fn rates_use_integer_differences_and_real_time() {
    let mut rate = RateBaseline::default();
    let big = 1_u64 << 60;
    assert!(rate.rates(&counters("a", big), 1000, 3000).is_err());
    assert_eq!(
        rate.rates(&counters("a", big + 750), 2500, 3000).unwrap(),
        (500.0, 500.0)
    );
    assert_eq!(
        rate.rates(&counters("a", big + 750), 3500, 3000).unwrap(),
        (0.0, 0.0)
    );
}
#[test]
fn rate_boundaries_rewarm() {
    let mut rate = RateBaseline::default();
    for (id, bytes, at) in [
        ("a", 100, 1000),
        ("a", 10, 2000),
        ("b", 500, 3000),
        ("b", 600, 20000),
        ("b", 700, 19000),
    ] {
        assert!(rate.rates(&counters(id, bytes), at, 3000).is_err());
    }
}
#[test]
fn a_stopped_producer_expires_current_snapshot_without_rewriting_history() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept(sample(1000, 0), 0);
    monitor.accept(sample(2000, 100), 0);
    assert_eq!(monitor.latest_at(5000).unwrap().cpu.status, Status::Normal);
    let expired = monitor.latest_at(5001).unwrap();
    assert_eq!(expired.cpu.status, Status::Stale);
    assert_eq!(expired.cpu.value, None);
    assert_eq!(expired.cpu.valid_at_ms, Some(12000));
    assert_eq!(expired.download.status, Status::Stale);
    assert_eq!(expired.memory_used, None);
    assert_eq!(monitor.history.back().unwrap().cpu.status, Status::Normal);
    monitor.accept(sample(6000, 200), 0);
    assert_eq!(monitor.latest_at(6000).unwrap().cpu.status, Status::Normal);
    assert_eq!(
        monitor.latest_at(6000).unwrap().download.status,
        Status::Warming
    );
}
#[test]
fn failure_does_not_refresh_valid_time_and_old_results_are_rejected() {
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.accept(sample(1000, 0), 0);
    let mut failed = sample(5001, 40);
    failed.cpu.result = Err(Failure::new(Status::Failed, "failure"));
    monitor.accept(failed, 0);
    let frame = monitor.history.back().unwrap();
    assert_eq!(frame.cpu.status, Status::Stale);
    assert_eq!(frame.cpu.valid_at_ms, Some(11000));
    assert_eq!(frame.cpu.value, None);
    assert_eq!(frame.download.status, Status::Warming);
    assert!(!monitor.accept(sample(6000, 50), 9));
    assert!(!monitor.accept(sample(4999, 50), 0));
}
#[test]
fn history_is_bounded_and_manual_selection_never_falls_back() {
    let mut monitor = Monitor::new(Settings::default(), None);
    for n in 1..1000 {
        monitor.accept(sample(n * 1000, n * 10), 0);
    }
    assert_eq!(monitor.history.len(), 301);
    monitor.settings.network_id = Some("removed".into());
    monitor.accept(sample(1000000, 10000), 0);
    assert!(monitor.selected.is_none());
    assert!(monitor.history.back().unwrap().download.value.is_none());
}
struct FailingRepository;
struct UnusedAutostart;
impl pinmeter_core::autostart::Autostart for UnusedAutostart {
    fn checkpoint(&self) -> Result<String, String> {
        panic!("unexpected autostart query")
    }
    fn restore(&self, _: &str) -> Result<(), String> {
        panic!("unexpected autostart change")
    }
    fn status(&self) -> pinmeter_core::autostart::AutostartStatus {
        panic!("unexpected autostart query")
    }
    fn set_enabled(&self, _: bool) -> Result<(), String> {
        panic!("unexpected autostart change")
    }
}
impl SettingsRepository for FailingRepository {
    fn load(&self) -> Result<Option<Settings>, String> {
        Ok(None)
    }
    fn save(&self, _: &Settings) -> Result<(), String> {
        Err("disk full".into())
    }
}
#[test]
fn failed_save_and_conflict_leave_authoritative_settings_unchanged() {
    let mut monitor = Monitor::new(Settings::default(), None);
    let settings = Settings {
        close_action: "exit".into(),
        interval_ms: 2000,
        ..Default::default()
    };
    assert!(
        monitor
            .update_settings(settings.clone(), 0, &FailingRepository, &UnusedAutostart)
            .is_err()
    );
    assert_eq!(monitor.settings.interval_ms, 1000);
    assert_eq!(monitor.settings.close_action, "ask");
    assert!(
        monitor
            .update_settings(settings, 1, &FailingRepository, &UnusedAutostart)
            .is_err()
    );
    assert_eq!(monitor.settings.revision, 0);
}
