// Adapted from zhihui-hu/one-ip, commit 5686f3f1. AGPL-3.0; see third-party/NOTICE.
use pinmeter_core::ip::*;
use serde_json::Value;
fn text(v: &Value) -> Option<String> {
    v.as_str()
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_control())
                .take(512)
                .collect::<String>()
        })
        .filter(|s| !s.trim().is_empty())
}
fn numeric(v: &Value) -> Option<String> {
    v.as_u64().map(|n| n.to_string())
}
pub fn adapt(data: &Value, address: &str) -> Result<IpProfile, IpError> {
    let address = public_address(address)?;
    if data["ip"]
        .as_str()
        .and_then(|s| public_address(s).ok())
        .as_deref()
        != Some(&address)
    {
        return Err(IpError::new(
            "address_mismatch",
            "资料返回的地址与查询地址不一致",
        ));
    }
    let score = data["trust_score"]
        .as_f64()
        .filter(|n| n.is_finite() && (0.0..=100.0).contains(n));
    let mut facts = Vec::new();
    for (key, upstream) in [
        ("cidr", "cidr"),
        ("reverse_dns", "rdns"),
        ("registered_country", "registered_country"),
        ("rpki", "rpki_status"),
        ("organization", "asOrganization"),
        ("asn_name", "asname"),
        ("company", "company_name"),
        ("company_type", "company_type"),
        ("datacenter_name", "datacenter_name"),
        ("asn_kind", "asn_kind"),
        ("asn_allocated", "asn_allocated"),
    ] {
        facts.push(IpFact {
            key: key.into(),
            value: text(&data[upstream]),
        });
    }
    for key in ["first", "last", "count"] {
        facts.push(IpFact {
            key: format!("range_{key}"),
            value: text(&data["range"][key]).or_else(|| numeric(&data["range"][key])),
        });
    }
    let flags = [
        ("residential", "isResidential"),
        ("datacenter", "is_datacenter"),
        ("mobile", "is_mobile"),
        ("vpn", "is_vpn"),
        ("proxy", "is_proxy"),
        ("tor", "is_tor"),
        ("crawler", "is_crawler"),
        ("abuser", "is_abuser"),
        ("public_service", "is_public_service"),
    ]
    .into_iter()
    .map(|(k, v)| IpFlag {
        key: k.into(),
        value: data[v].as_bool(),
    })
    .collect();
    let mut truncated = false;
    let list = |key: &str| data[key].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let locations = list("geo_sources")
        .iter()
        .take(100)
        .map(|g| IpLocation {
            source: format!(
                "Net.Coffee / {}",
                text(&g["src"]).unwrap_or_else(|| "未知".into())
            ),
            country: text(&g["country"]),
            region: text(&g["region"]),
            city: text(&g["city"]),
        })
        .collect();
    let mut records = Vec::new();
    for key in [
        "related_domains",
        "location_history",
        "asn_history",
        "company_history",
        "dc_neighbors",
    ] {
        if list(key).len() > 100 {
            truncated = true;
        }
        for r in list(key).iter().take(100) {
            let (value, detail) = match key {
                "related_domains" => (text(&r["domain"]), text(&r["via"])),
                "location_history" => (
                    text(&r["country"]),
                    Some(
                        [text(&r["region"]), text(&r["city"])]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" · "),
                    ),
                ),
                "asn_history" => (
                    numeric(&r["asn"]).map(|n| format!("AS{n}")),
                    text(&r["asn_org"]),
                ),
                "company_history" => (text(&r["company_name"]), text(&r["company_type"])),
                _ => (text(&r["ip"]), text(&r["company"])),
            };
            if let Some(value) = value {
                records.push(IpRecord {
                    kind: key.into(),
                    value,
                    detail,
                    at: numeric(&r["seen_at"]),
                });
            }
        }
    }
    truncated |= list("geo_sources").len() > 100;
    Ok(IpProfile {
        address,
        source: "Net.Coffee · one-ip 5686f3f1".into(),
        country: text(&data["country"]),
        region: text(&data["region"]),
        city: text(&data["city"]),
        isp: text(&data["isp"]),
        asn: data["asn"].as_u64().and_then(|n| u32::try_from(n).ok()),
        score,
        facts,
        flags,
        locations,
        records,
        truncated,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preserves_unknown_and_checks_identity() {
        let p = adapt(
            &json!({"ip":"1.1.1.1","trust_score":101,"is_proxy":"false","is_vpn":false}),
            "1.1.1.1",
        )
        .unwrap();
        assert_eq!(p.score, None);
        assert_eq!(
            p.flags.iter().find(|f| f.key == "proxy").unwrap().value,
            None
        );
        assert_eq!(
            p.flags.iter().find(|f| f.key == "vpn").unwrap().value,
            Some(false)
        );
        assert!(adapt(&json!({"ip":"8.8.8.8"}), "1.1.1.1").is_err());
    }
    #[test]
    fn score_and_ipv6() {
        for score in [0.0, 44.0, 45.0, 74.0, 75.0, 100.0] {
            assert_eq!(
                adapt(
                    &json!({"ip":"2606:4700:4700:0::1111","trust_score":score}),
                    "2606:4700:4700::1111"
                )
                .unwrap()
                .score,
                Some(score)
            );
        }
    }
}
