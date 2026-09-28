use pinmeter_core::{application::Monitor, domain::Settings};
fn main() {
    let full_state = std::env::args().any(|arg| arg == "--state");
    let count = std::env::args()
        .nth(1)
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(5)
        .min(3600);
    let mut provider = pinmeter_platform::provider();
    let mut monitor = Monitor::new(Settings::default(), None);
    monitor.cpu_model = pinmeter_platform::cpu_model();
    for n in 0..count {
        monitor.accept(provider.sample(), 0);
        if full_state {
            println!(
                "{}",
                serde_json::to_string(&pinmeter_host::presenters::state(
                    &monitor,
                    "probe",
                    u64::MAX
                ))
                .unwrap()
            );
        } else if let Some(frame) = monitor.history.back() {
            println!(
                "{}",
                serde_json::to_string(&pinmeter_host::presenters::frame(frame)).unwrap()
            );
        }
        if n + 1 < count {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
