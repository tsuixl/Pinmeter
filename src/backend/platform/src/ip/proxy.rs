use pinmeter_core::ip::IpError;
use std::{
    hash::{Hash, Hasher},
    time::Duration,
};
#[derive(Debug, Hash)]
struct Configuration {
    server: Option<String>,
    bypass: String,
    description: String,
}
pub fn route_signature() -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    match configuration() {
        Ok(c) => c.hash(&mut hash),
        Err(e) => {
            e.code.hash(&mut hash);
            e.detail.hash(&mut hash);
        }
    }
    hash.finish()
}
pub fn client() -> Result<(reqwest::Client, String), IpError> {
    let c = configuration()?;
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(1)
        .user_agent("Pinmeter/0.1.0 (IP inspection)");
    if let Some(server) = c.server {
        let server = if server.contains("://") {
            server
        } else {
            format!("http://{server}")
        };
        let proxy = reqwest::Proxy::https(server)
            .map_err(|_| IpError::new("proxy_config", "系统代理地址无效"))?
            .no_proxy(reqwest::NoProxy::from_string(&c.bypass.replace(';', ",")));
        builder = builder.proxy(proxy);
    }
    Ok((
        builder
            .build()
            .map_err(|_| IpError::new("http_init", "无法初始化安全网络连接"))?,
        c.description,
    ))
}
#[cfg(target_os = "windows")]
fn configuration() -> Result<Configuration, IpError> {
    use windows::Win32::{
        Foundation::{GlobalFree, HGLOBAL},
        Networking::WinHttp::{
            WINHTTP_CURRENT_USER_IE_PROXY_CONFIG, WinHttpGetIEProxyConfigForCurrentUser,
        },
    };
    let mut raw = WINHTTP_CURRENT_USER_IE_PROXY_CONFIG::default();
    unsafe { WinHttpGetIEProxyConfigForCurrentUser(&mut raw) }
        .map_err(|_| IpError::new("proxy_config", "无法读取当前应用账户的系统代理"))?;
    // WinHTTP owns no returned strings; the caller releases all three allocations.
    let read = |p: windows::core::PWSTR| {
        if p.is_null() {
            String::new()
        } else {
            unsafe { p.to_string() }.unwrap_or_default()
        }
    };
    let script = read(raw.lpszAutoConfigUrl);
    let server = read(raw.lpszProxy);
    let bypass = read(raw.lpszProxyBypass);
    for ptr in [raw.lpszAutoConfigUrl, raw.lpszProxy, raw.lpszProxyBypass] {
        if !ptr.is_null() {
            let _ = unsafe { GlobalFree(Some(HGLOBAL(ptr.0.cast()))) };
        }
    }
    if !script.is_empty() || raw.fAutoDetect.as_bool() {
        return Err(IpError::new(
            "unsupported",
            "当前系统使用 PAC/自动发现代理，本版尚不能确认其请求路由；未切换直连",
        ));
    }
    let selected = if server.contains('=') {
        server
            .split(';')
            .find_map(|entry| entry.trim().strip_prefix("https="))
            .map(str::to_owned)
    } else {
        Some(server).filter(|s| !s.trim().is_empty())
    };
    Ok(Configuration {
        description: if selected.is_some() {
            "当前账户的系统 HTTPS 代理（遵循例外规则）"
        } else {
            "系统路由（未配置 HTTPS 代理，可能经过 VPN/TUN）"
        }
        .into(),
        server: selected,
        bypass,
    })
}
#[cfg(not(target_os = "windows"))]
fn configuration() -> Result<Configuration, IpError> {
    let get = |upper: &str, lower: &str| {
        std::env::var(upper)
            .ok()
            .or_else(|| std::env::var(lower).ok())
            .filter(|s| !s.is_empty())
    };
    let server = get("HTTPS_PROXY", "https_proxy").or_else(|| get("ALL_PROXY", "all_proxy"));
    if server.is_none() {
        return Err(IpError::new(
            "unsupported",
            "尚未接入此平台的桌面代理解析；可识别 HTTPS_PROXY/ALL_PROXY 环境代理",
        ));
    }
    Ok(Configuration {
        server,
        bypass: get("NO_PROXY", "no_proxy").unwrap_or_default(),
        description: "环境变量代理（遵循 NO_PROXY）".into(),
    })
}
