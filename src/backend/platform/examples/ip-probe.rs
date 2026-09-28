//! Read-only IP smoke check. Never prints the machine's public address.
use pinmeter_core::ip::{ExitDetector, ExitTarget, IpProfileProvider};
fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let adapter = match pinmeter_platform::ip::IpAdapter::new() {
            Ok(a) => a,
            Err(e) => {
                println!("route: {} ({})", e.code, e.detail);
                return;
            }
        };
        if std::env::args().any(|a| a == "--checks") {
            use pinmeter_core::ip::checks::{CheckKind, CheckResult};
            for kind in CheckKind::ALL {
                for (index, id) in kind.targets().iter().enumerate() {
                    if let Some(target) = std::env::args()
                        .find_map(|a| a.strip_prefix("--target=").map(str::to_owned))
                        && target != *id
                    {
                        continue;
                    }
                    match adapter.check(kind, index).await {
                        CheckResult::Probe(Ok(report)) => println!(
                            "{} {id}: {} ms; HTTP={:?}",
                            kind.id(),
                            report.latency_ms,
                            report.http_status
                        ),
                        CheckResult::Service(Ok(report)) => println!(
                            "{} {id}: {}; incidents={}",
                            kind.id(),
                            report.state,
                            report.incidents.len()
                        ),
                        CheckResult::Probe(Err(e)) | CheckResult::Service(Err(e)) => {
                            println!("{} {id}: {} ({})", kind.id(), e.code, e.detail)
                        }
                    }
                }
            }
            return;
        }
        for target in ExitTarget::ALL {
            match adapter.detect(target).await {
                Ok(value) => println!(
                    "{}: valid public address; source={}; route={}",
                    target.id(),
                    value.source,
                    value.route
                ),
                Err(e) => println!("{}: {} ({})", target.id(), e.code, e.detail),
            }
        }
        match adapter.lookup("1.1.1.1").await {
            Ok(p) => println!(
                "public fixture: address matches={}, score present={}, flags={}, locations={}",
                p.address == "1.1.1.1",
                p.score.is_some(),
                p.flags.len(),
                p.locations.len()
            ),
            Err(e) => println!("public fixture: {} ({})", e.code, e.detail),
        }
    });
}
