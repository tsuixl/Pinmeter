fn main() {
    // Tauri emits its resources for binaries only. Integration tests constructing
    // a mock app still link TaskDialogIndirect and need Common Controls v6.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("app.manifest")
                .display()
        );
    }
    // Layered child windows require a Windows 8+ compatibility declaration.
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
    ))
    .expect("build desktop resources");
}
