pub mod app_network_control;
/// Build target identity only; runtime capabilities come from provider probes.
pub fn platform_name() -> &'static str {
    std::env::consts::OS
}

/// Static device metadata, read once during host composition rather than each sample.
pub fn cpu_model() -> Option<String> {
    #[cfg(target_os = "windows")]
    let model = windows::cpu_model();
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let model = sysinfo_provider::cpu_model();
    model
        .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|name| !name.is_empty())
}

pub mod app_network;
pub mod diagnostics;
pub mod disk;
pub mod gpu;
pub mod hardware;
pub mod ip;
pub mod settings;
pub mod shared;
pub mod startup;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod sysinfo_provider;
pub mod taskbar;
pub mod temperature;
pub mod temperature_driver;
#[cfg(target_os = "windows")]
mod windows;

pub fn provider() -> Box<dyn pinmeter_core::ports::MetricProvider> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsProvider::new())
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        Box::new(sysinfo_provider::SysinfoProvider::new())
    }
}
pub mod autostart;
pub mod updates;
