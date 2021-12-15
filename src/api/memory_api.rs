use crate::api::{ApiFuture, ReadApi};
use openapi_client::models;

pub struct MemoryApi {
    monitors: Vec<models::Monitor>,
    alerts: Vec<models::Alert>,
}

impl ReadApi for MemoryApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>> {
        let monitors = self.monitors.clone();

        Box::pin(async move { Ok(monitors) })
    }

    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>> {
        let alerts = self.alerts.clone();

        Box::pin(async move { Ok(alerts) })
    }
}
