use crate::api::*;
use crate::error::Error;
use crate::openapi_client::models;
use actix::prelude::*;
use futures::FutureExt;

use std::marker::Unpin;

pub struct ApiActor {
    api: Box<dyn Api + 'static>,
}

impl ApiActor {
    pub fn new<A: Api + 'static + Unpin>(api: A) -> Self {
        Self { api: Box::new(api) }
    }
}

impl Actor for ApiActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct MonitorsRequest {
    pub recipient: Recipient<MonitorsResponse>,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct MonitorsResponse {
    pub monitors: Vec<models::Monitor>,
}

pub struct RequestHandle {
    pub id: usize,
}

impl RequestHandle {
    pub fn new() -> Self {
        RequestHandle { id: 0 }
    }
}

impl Handler<MonitorsRequest> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, msg: MonitorsRequest, ctx: &mut Self::Context) -> Self::Result {
        let handle = RequestHandle::new();
        let addr = msg.recipient.clone();
        let future = self.api.get_monitors().map(move |m| match m {
            Ok(monitors) => {
                if let Err(err) = addr.do_send(MonitorsResponse { monitors }) {
                    error!("Error returning monitors: {}", err);
                }
            }
            Err(err) => {
                error!("Error returning monitors: {}", err);
            }
        });
        ctx.spawn(future.into_actor(self));
        Ok(handle)
    }
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct AlertsRequest {
    pub recipient: Recipient<AlertsResponse>,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct AlertsResponse {
    pub alerts: Vec<models::Alert>,
}

impl Handler<AlertsRequest> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, msg: AlertsRequest, ctx: &mut Self::Context) -> Self::Result {
        let addr = msg.recipient.clone();
        let future = self.api.get_alerts().map(move |a| match a {
            Ok(alerts) => {
                if let Err(err) = addr.do_send(AlertsResponse { alerts }) {
                    error!("Error returning alerts: {}", err);
                }
            }
            Err(err) => {
                error!("Error returning monitors: {}", err);
            }
        });
        ctx.spawn(future.into_actor(self));
        Ok(RequestHandle::new())
    }
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct HeartbeatRequest {
    pub recipient: Recipient<HeartbeatResponse>,
    pub session_id: String,
    pub agent_id: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct HeartbeatResponse {
    pub session: models::Session,
}

impl Handler<HeartbeatRequest> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, msg: HeartbeatRequest, ctx: &mut Self::Context) -> Self::Result {
        let addr = msg.recipient.clone();
        let future = self
            .api
            .post_heartbeat(&msg.session_id)
            .map(move |s| match s {
                Ok(session) => {
                    if let Err(err) = addr.do_send(HeartbeatResponse { session }) {
                        error!("Error returning session: {}", err);
                    }
                }
                Err(err) => {
                    error!("Error returning session: {}", err);
                }
            });
        ctx.spawn(future.into_actor(self));
        Ok(RequestHandle::new())
    }
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct StatusUpdatesRequest {
    pub recipient: Vec<Recipient<StatusUpdatesResponse>>,
    pub statuses: Vec<models::MonitorStatus>,
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct StatusUpdatesResponse {}

impl Handler<StatusUpdatesRequest> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, msg: StatusUpdatesRequest, ctx: &mut Self::Context) -> Self::Result {
        let handle = RequestHandle::new();
        let future = self.api.post_statuses(&msg.statuses).map(move |m| {
            match m {
                Ok(_) => {
                    // do nothing
                }
                Err(err) => {
                    error!("Error returning status update: {}", err);
                }
            }
        });
        ctx.spawn(future.into_actor(self));
        Ok(handle)
    }
}
