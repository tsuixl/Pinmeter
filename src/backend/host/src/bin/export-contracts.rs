fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/src/shared/contracts/monitor.ts");
    let expected = pinmeter_host::contracts::typescript();
    if std::env::args().any(|arg| arg == "--check") {
        assert_eq!(
            std::fs::read_to_string(path)
                .expect("Generate contracts first")
                .replace("\r\n", "\n"),
            expected,
            "Generated contracts differ"
        );
    } else {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, expected).unwrap();
    }
}
