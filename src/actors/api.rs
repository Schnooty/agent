use crate::api::*;
use crate::error::Error;
use actix::prelude::ResponseFuture;
use actix::prelude::*;
use openapi_client::models;

use std::marker::Unpin;
//use std::pin::Pin;

pub struct ApiActor<A: Api> {
    api: A,
}

impl<A: Api> ApiActor<A> {
    pub fn new(api: A) -> Self {
        Self { api }
    }
}

impl<A: Api + Unpin + 'static> Actor for ApiActor<A> {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<Vec<models::Monitor>, Error>")]
pub struct GetMonitors;

#[derive(Message)]
#[rtype(result = "Result<models::Session, Error>")]
pub struct SessionHeartbeat {
    pub group_id: String,
    pub session_id: String,
    pub agent_id: String,
}

impl<A: Api + Unpin + 'static> Handler<GetMonitors> for ApiActor<A> {
    type Result = ResponseFuture<Result<Vec<models::Monitor>, Error>>;

    fn handle(&mut self, _msg: GetMonitors, _ctx: &mut Self::Context) -> Self::Result {
        self.api.get_monitors()
        //Box::pin(actix::fut::wrap_future::<_, Self>(self.api.get_monitors()))
    }
}

impl<A: Api + Unpin + 'static> Handler<GetAlerts> for ApiActor<A> {
    type Result = ResponseFuture<Result<Vec<models::Alert>, Error>>;

    fn handle(&mut self, _msg: GetAlerts, _ctx: &mut Self::Context) -> Self::Result {
        self.api.get_alerts()
    }
}

impl<A: Api + Unpin + 'static> Handler<SessionHeartbeat> for ApiActor<A> {
    type Result = ResponseActFuture<Self, Result<models::Session, Error>>;

    fn handle(&mut self, msg: SessionHeartbeat, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(
            actix::fut::wrap_future::<_, Self>(
                self.api.post_heartbeat(&msg.group_id, &msg.session_id),
            )
            .map(|result, _, _| {
                debug!("Result from post_heartbeat: {:?}", result);
                result
            }),
        )
    }
}

impl<A: Api + Unpin + 'static> Handler<PostStatuses> for ApiActor<A> {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, msg: PostStatuses, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(
            actix::fut::wrap_future::<_, Self>(
                self.api.post_statuses(&msg.statuses),
            )
            .map(|result, _, _| {
                debug!("Result from post_heartbeat: {:?}", result);
                
                Ok(())
            }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<models::Alert>, Error>")]
pub struct GetAlerts;

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct PostStatusUpdate {
    pub status_updates: Vec<models::MonitorStatus>,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct PostHeartbeat {
    pub group_id: String,
    pub session_id: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct PostStatuses {
    pub statuses: Vec<models::MonitorStatus>,
}
