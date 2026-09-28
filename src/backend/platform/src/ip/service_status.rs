// Official sources and normalization adapted from one-ip status/services.json and
// worker/service-status.js, commit 5686f3f1 (AGPL-3.0). See third-party/NOTICE.
use super::{IpAdapter, http};
use pinmeter_core::ip::{
    IpError,
    checks::{ServiceItem, ServiceReport},
};
use serde_json::Value;
use std::time::Duration;

fn source(id: &str) -> Option<&'static str> {
    Some(match id {
        "openai" => "https://status.openai.com/api/v2/summary.json",
        "cloudflare" => "https://www.cloudflarestatus.com/api/v2/summary.json",
        "supabase" => "https://status.supabase.com/api/v2/summary.json",
        "claude" => "https://status.claude.com/api/v2/summary.json",
        "perplexity" => "https://status.perplexity.com/api/v2/summary.json",
        "cursor" => "https://status.cursor.com/api/v2/summary.json",
        "github" => "https://www.githubstatus.com/api/v2/summary.json",
        "vercel" => "https://www.vercel-status.com/api/v2/summary.json",
        _ => return None,
    })
}
fn text(v: &Value) -> Option<String> {
    v.as_str()
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_control())
                .take(256)
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
}
fn item(v: &Value) -> Option<ServiceItem> {
    Some(ServiceItem {
        name: text(&v["name"])?,
        state: text(&v["status"]).unwrap_or_else(|| "unknown".into()),
        updated_at: text(&v["updated_at"]).or_else(|| text(&v["updatedAt"])),
    })
}
fn adapt(data: &Value, source: &str) -> Result<ServiceReport, IpError> {
    let raw = data["status"]["indicator"]
        .as_str()
        .or_else(|| data["page"]["status"].as_str());
    let state = match raw {
        Some("none" | "UP") => "operational",
        Some("minor" | "HASISSUES") => "degraded",
        Some("major") => "partial_outage",
        Some("critical") => "major_outage",
        Some("maintenance" | "UNDERMAINTENANCE") => "maintenance",
        _ => return Err(IpError::new("invalid_response", "官方状态来源返回未知格式")),
    };
    let items = |key: &str| data[key].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let incidents = items("incidents")
        .iter()
        .chain(items("activeIncidents"))
        .chain(items("activeMaintenances"))
        .filter(|v| {
            !matches!(
                v["status"].as_str(),
                Some("resolved" | "completed" | "RESOLVED" | "COMPLETED")
            )
        })
        .filter_map(item)
        .take(30)
        .collect();
    Ok(ServiceReport {
        state: state.into(),
        source: source.into(),
        updated_at: text(&data["page"]["updated_at"]),
        incidents,
        components: items("components")
            .iter()
            .filter_map(item)
            .take(100)
            .collect(),
    })
}
impl IpAdapter {
    pub async fn service_status(&self, id: &str) -> Result<ServiceReport, IpError> {
        let source = source(id).ok_or_else(|| IpError::new("target", "未知服务来源"))?;
        let response = self
            .client
            .get(source)
            .timeout(Duration::from_secs(8))
            .send()
            .await
            .map_err(http::transport)?;
        adapt(&http::json(response).await?, source)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn unknown_never_becomes_operational_and_resolved_events_are_removed() {
        assert!(adapt(&json!({}), "fixture").is_err());
        assert!(adapt(&json!({"status":{"indicator":"new-status"}}), "fixture").is_err());
        let data = json!({"status":{"indicator":"minor"},"incidents":[{"name":"old","status":"resolved"},{"name":"incident","status":"investigating"}]});
        let r = adapt(&data, "fixture").unwrap();
        assert_eq!(r.state, "degraded");
        assert_eq!(r.incidents.len(), 1);
        assert_eq!(
            adapt(&json!({"page":{"status":"UNDERMAINTENANCE"}}), "fixture")
                .unwrap()
                .state,
            "maintenance"
        );
    }
}
