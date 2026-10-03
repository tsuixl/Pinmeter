fn main() {
    let mut collector = pinmeter_platform::disk::DiskCollector::new().expect("disk collector");
    let mut monitor = pinmeter_core::disk::DiskMonitor::default();
    for index in 0..3 {
        let samples = collector.sample();
        monitor.accept(samples, index * 2_000, index * 2_000);
        println!(
            "{} {:?}: {:?}",
            index,
            monitor.status,
            monitor.frames.back().map(|f| &f.disks)
        );
        if index < 2 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    assert!(
        monitor
            .frames
            .back()
            .unwrap()
            .disks
            .iter()
            .any(|d| d.read.status == pinmeter_core::domain::Status::Normal),
        "no valid disk read counter"
    );
}
