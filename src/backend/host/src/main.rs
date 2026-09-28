#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match pinmeter_platform::startup::prepare() {
        Ok(None) => (),
        Ok(Some(code)) => std::process::exit(code as i32),
        Err(error) => {
            pinmeter_platform::startup::report_error(&error);
            std::process::exit(1);
        }
    }
    pinmeter_host::run();
}
