use std::sync::atomic::AtomicBool;

fn main() {
    let cancel = AtomicBool::new(std::env::args().any(|arg| arg == "--cancel"));
    match pinmeter_platform::hardware::collect(&cancel) {
        pinmeter_core::hardware::HardwareState::Ready(inventory) => {
            println!("{}", serde_json::to_string_pretty(&inventory).unwrap());
        }
        other => {
            eprintln!("{other:?}");
            std::process::exit(1);
        }
    }
}
