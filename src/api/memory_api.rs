use crate::api::{ReadApi, ApiFuture};
use openapi_client::models;
use async_trait::async_trait;
use crate::error::Error;

pub struct MemoryApi {
    monitors: Vec<models::Monitor>,
    alerts: Vec<models::Alert>
}

impl MemoryApi {
    pub fn new(
        monitors: Vec<models::Monitor>,
        alerts: Vec<models::Alert>
    ) -> Self {
        Self {
            monitors,
            alerts
        }
    }
}

#[async_trait]
impl ReadApi for MemoryApi {
    async fn get_monitors(&self) -> Result<Vec<models::Monitor>, Error> {
        let monitors = self.monitors.clone();

        Ok(monitors)
    }

    async fn get_alerts(&self) -> Result<Vec<models::Alert>, Error> {
        let alerts = self.alerts.clone();

        Ok(alerts)
    }
}
