use openapi_client::models;
use std::future::Future;
use std::pin::Pin;
use crate::error::Error;

pub trait Api {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>>;
    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>>;
    fn post_heartbeat(&mut self, group_id: &str, session_id: &str) -> ApiFuture<models::Session>;
    fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> ApiFuture<()>;
}

pub type ApiFuture<T> = Pin<Box<dyn Future<Output = Result<T, Error>>>>;
