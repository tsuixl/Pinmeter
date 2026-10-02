use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};
use tauri_plugin_updater::UpdaterExt;

// Uses an ephemeral test signing key's PUBLIC artifacts only. No installer runs,
// no real endpoint is contacted and production transport rules are unchanged.
async fn download_fixture(tampered: bool) -> Result<Vec<u8>, tauri_plugin_updater::Error> {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/update-signature.json")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let endpoint = format!("{base}/latest.json");
    let bytes = fixture["payload"].as_str().unwrap().as_bytes().to_vec();
    let actual = if tampered {
        b"modified package".to_vec()
    } else {
        bytes
    };
    let body =
        serde_json::json!({"version":"999.0.0","notes":"测试版本 B","platforms":{"windows-x86_64":{
        "url":format!("{base}/package.exe"),"signature":fixture["signature"]}}})
        .to_string();
    let server = std::thread::spawn(move || {
        for payload in [body.into_bytes(), actual] {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer).unwrap();
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            )
            .unwrap();
            socket.write_all(&payload).unwrap();
        }
    });
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey":fixture["publicKey"],"dangerousInsecureTransportProtocol":true
        }),
    );
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let update = app
        .updater_builder()
        .target("windows-x86_64")
        .endpoints(vec![endpoint.parse().unwrap()])
        .unwrap()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .check()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "999.0.0");
    let result = update.download(|_, _| {}, || {}).await;
    server.join().unwrap();
    result
}
#[tokio::test]
async fn new_version_is_downloaded_and_signature_verified() {
    let bytes = download_fixture(false).await.unwrap();
    assert!(!bytes.is_empty());
}
#[tokio::test]
async fn modified_package_cannot_reach_install_ready() {
    assert!(download_fixture(true).await.is_err());
}
