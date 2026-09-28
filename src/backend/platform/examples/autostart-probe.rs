//! Explicit opt-in probe; always restores the exact pre-existing task.
use pinmeter_core::autostart::Autostart;
use pinmeter_platform::autostart::SystemAutostart;

fn main() -> Result<(), String> {
    let registration = SystemAutostart;
    let before = registration.status();
    println!("Initial status: {before:?}");
    if !std::env::args().any(|arg| arg == "--verify") {
        return Ok(());
    }
    let checkpoint = registration.checkpoint()?;
    let verify = (|| {
        registration.set_enabled(true)?;
        if registration.status().enabled != Some(true) {
            return Err("enable verification failed".into());
        }
        let xml = registration.checkpoint()?;
        if !(xml.contains("HighestAvailable")
            && xml.contains("InteractiveToken")
            && xml.contains("LogonTrigger")
            && xml.contains("PT0S"))
        {
            return Err("task definition verification failed".into());
        }
        registration.set_enabled(false)?;
        if registration.status().enabled != Some(false) {
            return Err("disable verification failed".into());
        }
        registration.set_enabled(false)?;
        Ok::<_, String>(())
    })();
    registration.restore(&checkpoint)?;
    assert_eq!(registration.status().enabled, before.enabled);
    println!("Original task restored; verification: {verify:?}");
    verify
}
