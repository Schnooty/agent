use crate::monitoring::MonitorFuture;
use openapi_client::models;

pub trait MonitorSource {
    fn type_name(&self) -> &'static str;
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture;
}
