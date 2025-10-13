use std::net::IpAddr;

use uuid::Uuid;

pub trait Verifier: Send + Sync + 'static {
    fn ask_for_approval(
        &self,
        req_id: Uuid,
        ip: IpAddr,
        commit: &str,
        size: usize,
    ) -> impl Future<Output = Option<bool>> + Send;
    fn report_error(&self, msg: &str) -> impl Future<Output = ()> + Send;
    fn report_success(&self, archive_name: &str) -> impl Future<Output = ()> + Send;
}
