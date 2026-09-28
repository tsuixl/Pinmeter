// Bounded request semantics adapted from one-ip public/worker/http.js. See third-party/NOTICE.
use pinmeter_core::ip::IpError;
use reqwest::{Response, StatusCode};
pub fn transport(error: reqwest::Error) -> IpError {
    if error.is_timeout() {
        IpError::new("timeout", "数据源请求超时")
    } else {
        IpError::new("connection", "无法连接数据源或代理")
    }
}
pub fn status(response: &Response) -> Result<(), IpError> {
    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        let delay = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.parse::<u64>().ok().or_else(|| {
                    httpdate::parse_http_date(v).ok().map(|d| {
                        d.duration_since(std::time::SystemTime::now())
                            .unwrap_or_default()
                            .as_secs()
                    })
                })
            })
            .unwrap_or(60);
        return Err(IpError {
            code: "rate_limited",
            detail: "数据源限流，请稍后重试".into(),
            retry_after_ms: delay.max(60).saturating_mul(1000),
        });
    }
    if !response.status().is_success() {
        return Err(IpError::new("upstream", "数据源暂不可用或拒绝请求"));
    }
    Ok(())
}
pub async fn json(mut response: Response) -> Result<serde_json::Value, IpError> {
    status(&response)?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(IpError::new("response_limit", "数据源响应超过大小限制"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| IpError::new("invalid_response", "数据源返回了无效内容"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    fn server(status: &str, headers: &str, body: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}\r\n",
            body.len()
        );
        let task = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        });
        (url, task)
    }
    #[test]
    fn bounded_transport_preserves_limits_and_bad_payloads() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap();
            for (status, header, body, expected) in [
                (
                    "429 Too Many Requests",
                    "Retry-After: 120\r\n",
                    vec![],
                    "rate_limited",
                ),
                ("200 OK", "", b"not json".to_vec(), "invalid_response"),
                (
                    "200 OK",
                    "",
                    vec![b' '; 2 * 1024 * 1024 + 1],
                    "response_limit",
                ),
                ("503 Unavailable", "", vec![], "upstream"),
            ] {
                let (url, task) = server(status, header, body);
                let error = json(client.get(url).send().await.unwrap())
                    .await
                    .unwrap_err();
                assert_eq!(error.code, expected);
                if expected == "rate_limited" {
                    assert_eq!(error.retry_after_ms, 120_000);
                }
                task.join().unwrap();
            }
        });
    }
}
