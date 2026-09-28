// Public-address validation adapted from one-ip public/worker/http.js (AGPL-3.0).
// Source, license and modifications: ../../../platform/src/ip/third-party/NOTICE.
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitTarget {
    Ipv4,
    Ipv6,
    Domestic,
}
impl ExitTarget {
    pub const ALL: [Self; 3] = [Self::Ipv4, Self::Ipv6, Self::Domestic];
    pub fn id(self) -> &'static str {
        match self {
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Domestic => "domestic",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryStatus {
    Idle,
    Loading,
    Ready,
    Failed,
    Stale,
    Unsupported,
}
impl QueryStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Stale => "stale",
            Self::Unsupported => "unsupported",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpError {
    pub code: &'static str,
    pub detail: String,
    pub retry_after_ms: u64,
}
impl IpError {
    pub fn new(code: &'static str, detail: &str) -> Self {
        Self {
            code,
            detail: detail.into(),
            retry_after_ms: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExitAddress {
    pub address: String,
    pub source: String,
    pub route: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpFact {
    pub key: String,
    pub value: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpFlag {
    pub key: String,
    pub value: Option<bool>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpLocation {
    pub source: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpRecord {
    pub kind: String,
    pub value: String,
    pub detail: Option<String>,
    pub at: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct IpProfile {
    pub address: String,
    pub source: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub asn: Option<u32>,
    pub score: Option<f64>,
    pub facts: Vec<IpFact>,
    pub flags: Vec<IpFlag>,
    pub locations: Vec<IpLocation>,
    pub records: Vec<IpRecord>,
    pub truncated: bool,
}
#[derive(Clone, Debug)]
pub struct Query<T> {
    pub status: QueryStatus,
    pub data: Option<T>,
    pub valid_at: Option<u64>,
    pub valid_mono: Option<u64>,
    pub error: Option<IpError>,
}
impl<T> Default for Query<T> {
    fn default() -> Self {
        Self {
            status: QueryStatus::Idle,
            data: None,
            valid_at: None,
            valid_mono: None,
            error: None,
        }
    }
}
impl<T> Query<T> {
    pub fn accept(&mut self, result: Result<T, IpError>, now: u64, wall: u64) {
        match result {
            Ok(data) => {
                self.data = Some(data);
                self.valid_at = Some(wall);
                self.valid_mono = Some(now);
                self.error = None;
                self.status = QueryStatus::Ready;
            }
            Err(error) => {
                self.status = if self.data.is_some() {
                    QueryStatus::Stale
                } else if error.code == "unsupported" {
                    QueryStatus::Unsupported
                } else {
                    QueryStatus::Failed
                };
                self.error = Some(error);
            }
        }
    }
    pub fn stale(&mut self) {
        self.status = if self.data.is_some() {
            QueryStatus::Stale
        } else {
            QueryStatus::Idle
        };
    }
}

/// Project validation: normalize equivalent addresses and reject non-public targets.
pub fn public_address(value: &str) -> Result<String, IpError> {
    let error = || IpError::new("invalid_address", "数据源未返回有效的公网地址");
    let mut ip: IpAddr = value.trim().parse().map_err(|_| error())?;
    if let IpAddr::V6(v6) = ip
        && let Some(v4) = v6.to_ipv4_mapped()
    {
        ip = IpAddr::V4(v4);
    }
    let valid = match ip {
        IpAddr::V4(v) => {
            let [a, b, c, _] = v.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 168 || b == 0 || (b == 88 && c == 99)))
                || (a == 100 && (64..=127).contains(&b))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(v) => {
            let s = v.segments();
            (0x2000..0x4000).contains(&s[0])
                && !(s[0] == 0x2001 && matches!(s[1], 0 | 2 | 0x10..=0x2f | 0xdb8))
                && !(s[0] == 0x2002)
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    };
    if valid {
        Ok(ip.to_string())
    } else {
        Err(error())
    }
}
