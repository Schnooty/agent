#![allow(warnings)]
use actix::prelude::*;
use crate::actors::*;
use std::fs::File;
use openapi_client::models;
use serde_json;
use crate::error::Error;
use std::io::prelude::*;

pub struct FileActor {
    path: String,
}

impl FileActor {
    pub fn new<S: ToString>(path: S) -> Self {
        Self {
            path: path.to_string()
        }
    }
}

impl Actor for FileActor {
    type Context = Context<Self>;
}

impl Handler<GetMonitors> for FileActor { //<A: Unpin + 'static>
    type Result = ResponseFuture<Result<Vec<models::Monitor>, Error>>;

    fn handle(&mut self, _msg: GetMonitors, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Getting monitors from {}", self.path);
        let path = self.path.clone();

        Box::pin(async move {
            let mut monitor_string = String::new();
            File::open(&path)?.read_to_string(&mut monitor_string)?;
            Ok(serde_json::from_str(&monitor_string)?)
        })
    }
}
