#![allow(warnings)]
use crate::actors::*;
use crate::error::Error;
use actix::prelude::*;
use openapi_client::models;
use serde_json;
use std::fs::File;
use std::io::prelude::*;

pub struct FileActor {
    path: String,
}

impl FileActor {
    pub fn new<S: ToString>(path: S) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl Actor for FileActor {
    type Context = Context<Self>;
}

impl Handler<MonitorsRequest> for FileActor {
    //<A: Unpin + 'static>
    type Result = Result<RequestHandle, Error>; //ResponseFuture<Result<Vec<models::Monitor>, Error>>;

    fn handle(&mut self, msg: MonitorsRequest, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Getting monitors from {}", self.path);
        let path = self.path.clone();
        let mut monitor_string = String::new();
        File::open(&path)?.read_to_string(&mut monitor_string)?;
        let monitors = serde_json::from_str(&monitor_string)?;
        msg.recipient.do_send(MonitorsResponse { monitors });
        Ok(RequestHandle::new())
    }
}
