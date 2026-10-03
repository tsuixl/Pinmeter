//! Read-only local DirectWrite inventory; never installs or copies system fonts.
fn main() {
    let catalog = pinmeter_platform::fonts::catalog();
    if std::env::args().any(|arg| arg == "--json") {
        println!("{}", serde_json::to_string_pretty(&catalog).unwrap());
    } else {
        println!(
            "{}; system_available={}",
            catalog.detail, catalog.system_available
        );
        for family in catalog.families.iter().filter(|f| {
            matches!(
                f.canonical_name.as_str(),
                "HarmonyOS Sans SC"
                    | "Arial"
                    | "Microsoft YaHei UI"
                    | "Segoe UI"
                    | "0xProto Nerd Font Mono"
            )
        }) {
            println!("{}", serde_json::to_string(family).unwrap());
        }
    }
    if cfg!(target_os = "windows") && !catalog.system_available {
        std::process::exit(1);
    }
}
