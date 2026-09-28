use pinmeter_core::ip::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpServiceItemDto {
    pub name: String,
    pub state: String,
    pub updated_at: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpCheckRowDto {
    pub id: String,
    pub status: String,
    pub valid_at_ms: Option<f64>,
    pub error: Option<String>,
    pub samples: Vec<Option<u32>>,
    pub latency_ms: Option<u32>,
    pub http_status: Option<u16>,
    pub service_state: Option<String>,
    pub source: Option<String>,
    pub updated_at: Option<String>,
    pub incidents: Vec<IpServiceItemDto>,
    pub components: Vec<IpServiceItemDto>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpCheckGroupDto {
    pub id: String,
    pub running: bool,
    pub retry_after_ms: f64,
    pub rows: Vec<IpCheckRowDto>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpFactDto {
    pub key: String,
    pub value: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpFlagDto {
    pub key: String,
    pub value: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpLocationDto {
    pub source: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpRecordDto {
    pub kind: String,
    pub value: String,
    pub detail: Option<String>,
    pub at: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpDetailsDto {
    pub address: String,
    pub source: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub asn: Option<u32>,
    pub score: Option<f64>,
    pub facts: Vec<IpFactDto>,
    pub flags: Vec<IpFlagDto>,
    pub locations: Vec<IpLocationDto>,
    pub records: Vec<IpRecordDto>,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpExitDto {
    pub id: String,
    pub status: String,
    pub address: Option<String>,
    pub source: String,
    pub route: String,
    pub valid_at_ms: Option<f64>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpProfileDto {
    pub address: String,
    pub status: String,
    pub valid_at_ms: Option<f64>,
    pub error: Option<String>,
    pub data: Option<IpDetailsDto>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct IpStateDto {
    pub checks: Vec<IpCheckGroupDto>,
    pub revision: String,
    pub generation: String,
    pub running: bool,
    pub retry_after_ms: f64,
    pub exits: Vec<IpExitDto>,
    pub profiles: Vec<IpProfileDto>,
}
fn details(p: &IpProfile) -> IpDetailsDto {
    IpDetailsDto {
        address: p.address.clone(),
        source: p.source.clone(),
        country: p.country.clone(),
        region: p.region.clone(),
        city: p.city.clone(),
        isp: p.isp.clone(),
        asn: p.asn,
        score: p.score,
        facts: p
            .facts
            .iter()
            .map(|f| IpFactDto {
                key: f.key.clone(),
                value: f.value.clone(),
            })
            .collect(),
        flags: p
            .flags
            .iter()
            .map(|f| IpFlagDto {
                key: f.key.clone(),
                value: f.value,
            })
            .collect(),
        locations: p
            .locations
            .iter()
            .map(|l| IpLocationDto {
                source: l.source.clone(),
                country: l.country.clone(),
                region: l.region.clone(),
                city: l.city.clone(),
            })
            .collect(),
        records: p
            .records
            .iter()
            .map(|r| IpRecordDto {
                kind: r.kind.clone(),
                value: r.value.clone(),
                detail: r.detail.clone(),
                at: r.at.clone(),
            })
            .collect(),
        truncated: p.truncated,
    }
}
pub fn snapshot(ip: &IpInspection) -> IpStateDto {
    IpStateDto {
        checks: ip
            .checks
            .iter()
            .map(|g| IpCheckGroupDto {
                id: g.kind.id().into(),
                running: g.running,
                retry_after_ms: g.retry_at.saturating_sub(super::now()) as f64,
                rows: g
                    .rows
                    .iter()
                    .map(|r| {
                        let service = match &r.query.data {
                            Some(checks::CheckData::Service(s)) => Some(s),
                            _ => None,
                        };
                        let item = |v: &checks::ServiceItem| IpServiceItemDto {
                            name: v.name.clone(),
                            state: v.state.clone(),
                            updated_at: v.updated_at.clone(),
                        };
                        IpCheckRowDto {
                            id: r.id.into(),
                            status: r.query.status.id().into(),
                            valid_at_ms: r.query.valid_at.map(|t| t as f64),
                            error: r
                                .query
                                .error
                                .as_ref()
                                .map(|e| e.detail.clone())
                                .or_else(|| r.note.clone()),
                            samples: r.samples.clone(),
                            latency_ms: match &r.query.data {
                                Some(checks::CheckData::Probe(report)) => Some(report.latency_ms),
                                _ => None,
                            },
                            http_status: match &r.query.data {
                                Some(checks::CheckData::Probe(report)) => report.http_status,
                                _ => None,
                            },
                            service_state: service.map(|s| s.state.clone()),
                            source: service.map(|s| s.source.clone()),
                            updated_at: service.and_then(|s| s.updated_at.clone()),
                            incidents: service
                                .map_or_else(Vec::new, |s| s.incidents.iter().map(item).collect()),
                            components: service
                                .map_or_else(Vec::new, |s| s.components.iter().map(item).collect()),
                        }
                    })
                    .collect(),
            })
            .collect(),
        revision: ip.revision.to_string(),
        generation: ip.generation.to_string(),
        running: ip.running,
        retry_after_ms: ip.retry_at.saturating_sub(super::now()) as f64,
        exits: ip
            .exits
            .iter()
            .map(|(t, q)| IpExitDto {
                id: t.id().into(),
                status: q.status.id().into(),
                address: q.data.as_ref().map(|d| d.address.clone()),
                source: q.data.as_ref().map_or(String::new(), |d| d.source.clone()),
                route: q.data.as_ref().map_or(String::new(), |d| d.route.clone()),
                valid_at_ms: q.valid_at.map(|t| t as f64),
                error: q.error.as_ref().map(|e| e.detail.clone()),
            })
            .collect(),
        profiles: ip
            .profiles
            .iter()
            .filter(|p| {
                ip.exits
                    .iter()
                    .any(|(_, q)| q.data.as_ref().is_some_and(|d| d.address == p.address))
            })
            .map(|p| IpProfileDto {
                address: p.address.clone(),
                status: p.query.status.id().into(),
                valid_at_ms: p.query.valid_at.map(|t| t as f64),
                error: p.query.error.as_ref().map(|e| e.detail.clone()),
                data: p.query.data.as_ref().map(details),
            })
            .collect(),
    }
}
pub fn declarations(config: &ts_rs::Config) -> Vec<String> {
    vec![
        IpServiceItemDto::decl(config),
        IpCheckRowDto::decl(config),
        IpCheckGroupDto::decl(config),
        IpFactDto::decl(config),
        IpFlagDto::decl(config),
        IpLocationDto::decl(config),
        IpRecordDto::decl(config),
        IpDetailsDto::decl(config),
        IpExitDto::decl(config),
        IpProfileDto::decl(config),
        IpStateDto::decl(config),
    ]
}
