use pinmeter_core::{ports::Clock, processes::ProcessRanking};
fn main() {
    let clock = pinmeter_platform::shared::SystemClock::default();
    let mut ranking = ProcessRanking::default();
    for i in 0..2 {
        let start = std::time::Instant::now();
        ranking.accept(
            pinmeter_platform::processes::collect().expect("process snapshot"),
            clock.monotonic_ms(),
        );
        println!(
            "sample={i} processes={} unreadable={} elapsed_ms={}",
            ranking.rows.len(),
            ranking.unreadable,
            start.elapsed().as_millis()
        );
        if i == 0 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    assert!(!ranking.top(false).is_empty());
    assert!(!ranking.top(true).is_empty());
    // Do not emit other applications' names, PIDs or paths in validation logs.
    let own = ranking
        .rows
        .iter()
        .find(|r| r.pid == std::process::id())
        .expect("probe process");
    assert!(own.working_set.is_some());
    assert!(own.cpu.is_some());
    println!("Top 10 and current-process CPU/working-set readings valid");
}
