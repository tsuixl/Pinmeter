// Targets adapted from one-ip connectivity.tsx and ai/probe.ts (5686f3f1).
// AGPL-3.0; see third-party/NOTICE. No credentials or remote scripts.
use super::{IpAdapter, http};
use pinmeter_core::ip::{IpError, checks::*};
use std::time::{Duration, Instant};

fn target(kind: CheckKind, id: &str) -> Option<(&'static str, bool)> {
    let url = match (kind, id) {
        (CheckKind::Connectivity, "bytedance") => "https://perfops.byte-test.com/500b-bench.jpg",
        (CheckKind::Connectivity, "wechat") => {
            "https://res.wx.qq.com/a/wx_fed/assets/res/NTI4MWU5.ico"
        }
        (CheckKind::Connectivity, "taobao") => {
            "https://gw.alicdn.com/imgextra/i4/O1CN01qOI6vB1zaqrBKbyFr_!!6000000006731-73-tps-64-64.ico"
        }
        (CheckKind::Connectivity, "youtube") => "https://www.youtube.com/generate_204",
        (CheckKind::Connectivity, "cloudflare") => "https://1.1.1.1/cdn-cgi/trace",
        (CheckKind::Connectivity, "github") => "https://github.com/robots.txt",
        (CheckKind::Ai, "deepseek") => "https://chat.deepseek.com/favicon.ico",
        (CheckKind::Ai, "qwen") => "https://chat.qwen.ai/favicon.ico",
        (CheckKind::Ai, "kimi") => "https://www.kimi.com/favicon.ico",
        (CheckKind::Ai, "chatgpt") => "https://chatgpt.com/favicon.ico",
        (CheckKind::Ai, "claude") => "https://claude.ai/cdn-cgi/trace",
        (CheckKind::Ai, "perplexity") => "https://www.perplexity.ai/cdn-cgi/trace",
        (CheckKind::Ai, "grok") => "https://grok.com/robots.txt",
        (CheckKind::Ai, "gemini") => "https://gemini.google.com/robots.txt",
        _ => return None,
    };
    Some((url, url.ends_with("/cdn-cgi/trace")))
}

impl IpAdapter {
    pub async fn check(&self, kind: CheckKind, index: usize) -> CheckResult {
        let id = kind.targets().get(index).copied().unwrap_or("");
        if kind == CheckKind::Services {
            return CheckResult::Service(self.service_status(id).await);
        }
        CheckResult::Probe(self.probe(kind, id).await)
    }
    async fn probe(&self, kind: CheckKind, id: &str) -> Result<ProbeReport, IpError> {
        let (url, trace) =
            target(kind, id).ok_or_else(|| IpError::new("target", "未知检测目标"))?;
        self.probe_url(url, trace, kind == CheckKind::Ai && !trace)
            .await
    }
    async fn probe_url(
        &self,
        url: &str,
        trace: bool,
        headers_only: bool,
    ) -> Result<ProbeReport, IpError> {
        let start = Instant::now();
        let mut response = self
            .client
            .get(url)
            .header("Cache-Control", "no-cache")
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .map_err(http::transport)?;
        let http_status = Some(response.status().as_u16());
        if headers_only {
            // one-ip's opaque probe resolves at headers, without reading status/body.
            // Native HTTP can additionally retain status without equating 403 to offline.
            let retry_after_ms = if response.status().as_u16() == 429 {
                http::status(&response).unwrap_err().retry_after_ms
            } else {
                0
            };
            return Ok(ProbeReport {
                latency_ms: start.elapsed().as_millis().min(u32::MAX as u128) as u32,
                http_status,
                retry_after_ms,
            });
        }
        if response.status().as_u16() == 429 {
            http::status(&response)?;
        }
        if !response.status().is_success() {
            return Err(IpError::new(
                "restricted",
                &format!(
                    "目标返回 HTTP {}，未确认资源可访问",
                    response.status().as_u16()
                ),
            ));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(http::transport)? {
            if body.len() + chunk.len() > 256 * 1024 {
                return Err(IpError::new("response_limit", "探测资源超过大小限制"));
            }
            body.extend_from_slice(&chunk);
        }
        if trace {
            let text = std::str::from_utf8(&body)
                .map_err(|_| IpError::new("invalid_response", "目标响应不是有效 trace"))?;
            if !text.lines().any(|l| {
                l.strip_prefix("ip=")
                    .is_some_and(|v| super::public_address(v).is_ok())
            }) {
                return Err(IpError::new(
                    "invalid_response",
                    "目标未返回有效的边缘网络响应",
                ));
            }
        }
        Ok(ProbeReport {
            latency_ms: start.elapsed().as_millis().min(u32::MAX as u128) as u32,
            http_status,
            retry_after_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn opaque_ai_counts_headers_but_trace_and_network_stay_strict() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let adapter = IpAdapter {
                client: reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap(),
                route: "fixture".into(),
            };
            for (status, headers_only, trace, expected_error) in [
                (200, true, false, None),
                (302, true, false, None),
                (403, true, false, None),
                (404, true, false, None),
                (429, true, false, None),
                (503, true, false, None),
                (403, false, false, Some("restricted")),
                (429, false, false, Some("rate_limited")),
                (200, false, true, Some("connection")),
            ] {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let url = format!("http://{}", listener.local_addr().unwrap());
                let task = std::thread::spawn(move || {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    let mut buffer = [0; 4096];
                    let _ = stream.read(&mut buffer);
                    // Deliberately incomplete body: opaque probes must finish at headers.
                    write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Length: 100\r\nRetry-After: 120\r\nConnection: close\r\n\r\n").unwrap();
                });
                let result = adapter.probe_url(&url, trace, headers_only).await;
                if let Some(code) = expected_error {
                    assert_eq!(result.unwrap_err().code, code);
                } else {
                    let report = result.unwrap();
                    assert_eq!(report.http_status, Some(status));
                    assert_eq!(report.retry_after_ms, if status == 429 {120_000} else {0});
                }
                task.join().unwrap();
            }
            // A closed local port produces no HTTP observation, never a zero-ms success.
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            drop(listener);
            assert_eq!(adapter.probe_url(&url, false, true).await.unwrap_err().code, "connection");
        });
    }
}
