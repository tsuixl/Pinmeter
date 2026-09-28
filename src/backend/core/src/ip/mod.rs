mod model;
mod service;
pub use model::*;
pub use service::*;

use std::future::Future;
pub trait ExitDetector {
    fn detect(
        &self,
        target: ExitTarget,
    ) -> impl Future<Output = Result<ExitAddress, IpError>> + Send;
}
pub trait IpProfileProvider {
    fn lookup(&self, address: &str) -> impl Future<Output = Result<IpProfile, IpError>> + Send;
}
pub mod checks;
