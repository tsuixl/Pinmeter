//! Development-only real native display probe; does not start privileged collectors.
use pinmeter_core::desktop::*;
use pinmeter_platform::taskbar::Taskbar;
fn main() {
    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(20)
        .min(300);
    let taskbar = Taskbar::start();
    let mut settings = TaskbarSettings {
        enabled: true,
        ..Default::default()
    };
    let mut revision = 0;
    for second in 0..seconds {
        // Exercise both orientations and explicit hide/recovery with a single worker.
        let previous = settings.clone();
        if second == seconds / 3 {
            settings.hidden = true;
        }
        if second == seconds / 3 + 2 {
            settings.hidden = false;
        }
        if second == seconds / 2 {
            settings.layout = "single".into();
        }
        if second == seconds.saturating_sub(3) {
            settings.hidden = true;
        }
        if previous != settings {
            revision += 1;
        }
        taskbar.submit(DesktopSummary {
            session: "native-probe".into(),
            cursor: second,
            network_id: None,
            gpu_id: Some("example-gpu".into()),
            revision,
            settings: settings.clone(),
            network: "布局验证 · 示例数据，不是实际采样".into(),
            readings: [
                ("network", "下载", "3.2", "MB/s"),
                ("network", "上传", "128.0", "KB/s"),
                ("cpu", "CPU", "12", "%"),
                ("memory", "内存", "46", "%"),
                ("gpu", "GPU", "18", "%"),
            ]
            .into_iter()
            .map(|(key, label, text, unit)| SummaryReading {
                valid_at_ms: None,
                key,
                label,
                text: text.into(),
                unit: unit.into(),
                detail: "示例读数".into(),
                normal: true,
                temperature: matches!(key, "cpu" | "gpu").then(|| SummaryTemperature {
                    text: if key == "cpu" { "57" } else { "49" }.into(),
                    normal: true,
                    detail: "示例温度".into(),
                    valid_at_ms: None,
                    alternative_sensor: false,
                }),
            })
            .collect(),
        });
        while let Ok(action) = taskbar.actions.try_recv() {
            println!("action={action:?}");
        }
        println!("second={second} status={:?}", taskbar.status());
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    drop(taskbar);
    println!("Native taskbar stopped and joined.");
}
