use crate::api::*;
use crate::error::Error;
use actix::prelude::ResponseFuture;
use actix::prelude::*;
use openapi_client::models;
use async_trait::async_trait;
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
pub struct GetMonitors;

pub struct RequestHandle {
    pub id: usize
}

impl RequestHandle {
    fn new() -> Self {
        RequestHandle {
            id: 0
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct SessionHeartbeat {
    pub session_id: String,
    pub agent_id: String,
}

#[derive(Message)]
#[rtype(result = "Result<RequestHandle, Error>")]
pub struct GetAlerts;

impl Handler<GetMonitors> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, _msg: GetMonitors, ctx: &mut Self::Context) -> Self::Result {
        let handle = RequestHandle::new();
        let future = self.api.get_monitors()
            .map(|_| ());
        ctx.spawn(future.into_actor(self));
        Ok(handle)
        //Box::pin(actix::fut::wrap_future::<_, _>())
        //Box::pin(actix::fut::wrap_future::<_, _>(self.api.get_monitors()))
        //Box::pin(self.api.get_monitors()))
        //let monitor_future = self.api.get_monitors();
        //monitor_future
        //Box::pin(async {
        //    Ok(Vec::new())
        //})
        //.into_actor(self)
        /*Box::pin(
            //async {
                // Some async computation
            //    42
            //}
            monitor_future
            .into_actor(self) // converts future to ActorFuture
            .map(|_res, _act, _ctx| {
                // Do some computation with actor's state or context
                Ok(vec![])
            }),
        )*/
        //let monitors = self.api.get_monitors();
        //Box::pin(
        //    monitors
        //    .into_actor(self) // converts future to ActorFuture
        //)
        //Box::pin(async {
        //    Ok(monitors.await)
        //})
    }
}

impl Handler<GetAlerts> for ApiActor {
    type Result = Result<RequestHandle, Error>;

    fn handle(&mut self, _msg: GetAlerts, ctx: &mut Self::Context) -> Self::Result {
        let alerts = self.api.get_alerts();
        //Box::pin(
        //   alerts 
        //    .into_actor(self) // converts future to ActorFuture
        //)
    }
}

impl Handler<SessionHeartbeat> for ApiActor {
    type Result = ();//ResponseActFuture<Self, Result<models::Session, Error>>;

    fn handle(&mut self, msg: SessionHeartbeat, _ctx: &mut Self::Context) -> Self::Result {
        //Box::pin(actix::fut::wrap_future::<_, Self>(self.api.post_heartbeat(&msg.session_id)))
        //todo!()
    }
}

impl Handler<PostStatuses> for ApiActor {
    type Result = ();//ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, msg: PostStatuses, _ctx: &mut Self::Context) -> Self::Result {
        //Box::pin(actix::fut::wrap_future::<_, Self>(self.api.post_statuses(&msg.statuses)))
        todo!()
    }
}

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
