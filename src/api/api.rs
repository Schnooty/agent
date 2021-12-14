use crate::error::Error;
use openapi_client::models;
use std::future::Future;
use std::pin::Pin;

pub trait ReadApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>>;
    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>>;
}

pub trait Api: ReadApi {
    fn post_heartbeat(&mut self, session_id: &str) -> ApiFuture<models::Session>;
    fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> ApiFuture<()>;
}

pub type ApiFuture<T> = Pin<Box<dyn Future<Output = Result<T, Error>>>>;
