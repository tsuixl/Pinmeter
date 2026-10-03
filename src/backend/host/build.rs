fn main() {
    // Tauri emits resources for binaries only. Both integration tests and the
    // library's unit-test harness link TaskDialogIndirect and need Common Controls v6.
    // The -tests variant misses the library harness, so apply linker arguments
    // to every linked target; normal rlibs do not invoke the linker.
    let msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if msvc {
        println!("cargo:rerun-if-changed=app.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("app.manifest")
                .display()
        );
    }
    // Layered child windows require a Windows 8+ compatibility declaration.
    // MSVC embeds the exact same manifest through the linker for every target.
    // Do not also emit RT_MANIFEST in resource.lib: that duplicates resource #1.
    let windows = if msvc {
        tauri_build::WindowsAttributes::new_without_app_manifest()
    } else {
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"))
    };
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("build desktop resources");
}
