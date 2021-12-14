use openapi_client::models;
use std::future::Future;
use std::pin::Pin;
use crate::error::Error;
use async_trait::async_trait;

#[async_trait]
pub trait ReadApi {
    async fn get_monitors(&self) -> Result<Vec<models::Monitor>, Error>;
    async fn get_alerts(&self) -> Result<Vec<models::Alert>, Error>;
}

#[async_trait]
pub trait Api: ReadApi {
    async fn post_heartbeat(&mut self, session_id: &str) -> Result<models::Session, Error>;
    async fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> Result<(), Error>;
}

pub type ApiFuture<T> = Pin<Box<dyn Future<Output = Result<T, Error>>>>;
