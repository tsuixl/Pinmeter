// Detection adapted from one-ip src/views/home/api.ts; see third-party/NOTICE.
mod checks;
mod coffee;
mod http;
mod proxy;
mod service_status;
use pinmeter_core::ip::*;
pub use proxy::route_signature;
use std::time::Duration;
pub struct IpAdapter {
    client: reqwest::Client,
    route: String,
}
impl IpAdapter {
    pub fn new() -> Result<Self, IpError> {
        let (client, route) = proxy::client()?;
        Ok(Self { client, route })
    }
    async fn detect_inner(&self, target: ExitTarget) -> Result<ExitAddress, IpError> {
        if target != ExitTarget::Domestic {
            let url = if target == ExitTarget::Ipv4 {
                "https://api4.ipify.org?format=json"
            } else {
                "https://api6.ipify.org?format=json"
            };
            let data = http::json(
                self.client
                    .get(url)
                    .timeout(Duration::from_secs(3))
                    .send()
                    .await
                    .map_err(http::transport)?,
            )
            .await?;
            let address = public_address(data["ip"].as_str().unwrap_or(""))?;
            if address.contains(':') != (target == ExitTarget::Ipv6) {
                return Err(IpError::new(
                    "address_family",
                    "检测目标未返回预期类型的地址",
                ));
            }
            return Ok(ExitAddress {
                address,
                source: if target == ExitTarget::Ipv4 {
                    "api4.ipify.org"
                } else {
                    "api6.ipify.org"
                }
                .into(),
                route: self.route.clone(),
            });
        }
        let mut error = IpError::new("missing_header", "国内目标未返回可读取的出口地址");
        for (url, header, source) in [
            (
                "https://necaptcha.nosdn.127.net/ab7f4275c1744aa28e0a8f3a1c58c532.png",
                "cdn-user-ip",
                "necaptcha.nosdn.127.net",
            ),
            (
                "https://perfops.byte-test.com/500b-bench.jpg",
                "x-request-ip",
                "perfops.byte-test.com",
            ),
        ] {
            match self
                .client
                .head(url)
                .timeout(Duration::from_secs(3))
                .send()
                .await
            {
                Ok(r) => {
                    if let Err(e) = http::status(&r) {
                        if e.code == "rate_limited" {
                            return Err(e);
                        }
                        error = e;
                        continue;
                    }
                    if let Some(address) = r
                        .headers()
                        .get(header)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| public_address(v).ok())
                        .filter(|v| !v.contains(':'))
                    {
                        return Ok(ExitAddress {
                            address,
                            source: source.into(),
                            route: self.route.clone(),
                        });
                    }
                }
                Err(e) => {
                    error = http::transport(e);
                }
            }
        }
        Err(error)
    }
}
impl ExitDetector for IpAdapter {
    async fn detect(&self, target: ExitTarget) -> Result<ExitAddress, IpError> {
        tokio::time::timeout(Duration::from_secs(6), self.detect_inner(target))
            .await
            .map_err(|_| IpError::new("timeout", "出口检测超时"))?
    }
}
impl IpProfileProvider for IpAdapter {
    async fn lookup(&self, address: &str) -> Result<IpProfile, IpError> {
        let address = public_address(address)?;
        let url = format!("https://ip.net.coffee/api/ip/lookup/{address}");
        let data = http::json(self.client.get(url).send().await.map_err(http::transport)?).await?;
        coffee::adapt(&data, &address)
    }
}
