use crate::actors::*;
use actix::Context;
use std::time::Duration;
use crate::error::Error;
use actix::prelude::SpawnHandle;
use crate::api::{HttpApi, HttpConfig};
use openapi_client::models;
use crate::api::Api;

const HEARTBEAT_DURATION_SEC: u64 = 30;

pub struct SessionActor {
    heartbeat_handle: Option<SpawnHandle>,
    session_recipients: Vec<Recipient<SessionInfoMsg>>
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct SessionInfoMsg {
    pub session: models::Session
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct SessionState {
}

impl SessionActor {
    pub fn new(session_recipients: Vec<Recipient<SessionInfoMsg>>) -> Self {
        Self {
            session_recipients,
            heartbeat_handle: None
        }
    }
}

impl Actor for SessionActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("SessionActor stopped");
    }
}

impl Handler<CurrentConfig> for SessionActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, config_msg: CurrentConfig, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling latest config for session");

        // first cancel the heartbeat process
        let mut current_handle = None;
        std::mem::swap(&mut current_handle, &mut self.heartbeat_handle);

        if let Some(handle) = current_handle {
            debug!("Cancelling heartbeat interval");
            ctx.cancel_future(handle);
        }

        let base_url = match config_msg.config.base_url {
            Some(ref u) => u.clone(),
            None => {
                debug!("Base URL not set. Session will not be initialised with API.");
                return Ok(());
            }
        };

        debug!("Using base url {} to initialise session", base_url);

        // initalise the config 
        let http_config = HttpConfig {
            base_url,
            api_key: config_msg.config.api_key.clone()
        };

        let session_id = config_msg.config.session.name.clone();

        let session_msg = SessionTimeout {
            http_config,
            session_id
        };

        ctx.address().do_send(session_msg.clone());

        debug!("Setting up heartbeat interval");
        let heartbeat_process = move |_: &mut SessionActor, ctx: &mut Context<Self>| {
            ctx.address().do_send(session_msg.clone());
        };
        let heartbeat_duration = Duration::new(60, 0);
        self.heartbeat_handle = Some(ctx.run_interval(heartbeat_duration, heartbeat_process));

        Ok(())
    }
}

impl Handler<SessionTimeout> for SessionActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, timeout: SessionTimeout, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Ready to send heartbeat now");

        let api = HttpApi::new(&timeout.http_config);
        let sid = timeout.session_id;
        let recipients = self.session_recipients.clone();

        let mut api = api.clone();

        debug!("Sending heartbeat now");

        let heartbeat_future = async move {
            match api.post_heartbeat(&sid).await {
                Ok(session) => {
                    debug!("Heartbeat sent successfully");

                    for recipient in recipients {
                        if let Err(err) = recipient.do_send(SessionInfoMsg {
                            session: session.clone()
                        }) {
                            error!("Error sending session info msg: {}", err);
                        }
                    }
                },
                Err(err) => error!("Error posting heartbeat: {}", err)
            }
        };

        ctx.spawn(actix::fut::wrap_future(heartbeat_future));
        
        Ok(())
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
struct SessionTimeout {
    session_id: String,
    http_config: HttpConfig
}
