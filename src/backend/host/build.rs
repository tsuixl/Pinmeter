fn main() {
    // Layered child windows require a Windows 8+ compatibility declaration.
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
    ))
    .expect("build desktop resources");
}
