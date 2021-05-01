use crate::actors::ConfiguratorActor;
use crate::actors::*;
use crate::api::HttpApi;
use actix::clock::Duration;
use actix::ResponseActFuture;
use chrono::prelude::*;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::error;
use std::iter;

const HEARTBEAT_DURATION_SEC: u64 = 30;

pub struct AgentGroupInfo {
    pub agent_id: String,
    pub group_id: String,
}

pub struct SessionActor {
    agent_group_info: AgentGroupInfo,
    session_id: String,
    api_addr: Addr<ApiActor<HttpApi>>,
    configurator_addr: Addr<ConfiguratorActor>,
}

impl SessionActor {
    pub fn new(
        agent_id: &str,
        group_id: &str,
        api_addr: Addr<ApiActor<HttpApi>>,
        configurator_addr: Addr<ConfiguratorActor>,
    ) -> Self {
        let mut rng = thread_rng();
        let session_id: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(SESSION_ID_LEN)
            .collect();

        Self {
            agent_group_info: AgentGroupInfo {
                agent_id: agent_id.to_owned(),
                group_id: group_id.to_owned(),
            },
            session_id,
            api_addr,
            configurator_addr,
        }
    }

    /*pub fn schedule_heartbeat(&mut self, ctx: &mut Context<Self>) {
        let heartbeat = Heartbeat {
            session_id: self.session_id.to_owned(),
        };
        ctx.run_later(
            Duration::new(HEARTBEAT_DURATION_SEC, 0),
            move |act, mut ctx| {
                act.schedule_heartbeat(&mut ctx);

                ctx.spawn(
                    actix::fut::wrap_future::<_, Self>(ctx.address().send(heartbeat))
                        .map(|_, _, _| ()),
                );
            },
        );
    }*/
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), SessionErr>")]
pub struct SessionInit {}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), SessionErr>")]
pub struct Heartbeat {
    pub session_id: String,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), SessionErr>")]
struct LoopedHeartbeat {}

#[derive(Debug)]
pub struct SessionErr {
    pub error: Option<Box<dyn error::Error + std::marker::Send>>,
}

const SESSION_ID_LEN: usize = 32;

impl Handler<SessionInit> for SessionActor {
    type Result = ResponseActFuture<Self, Result<(), SessionErr>>;

    fn handle(&mut self, _msg: SessionInit, ctx: &mut Context<Self>) -> Self::Result {
        info!("Starting a new session");

        debug!("agent_id={}", self.agent_group_info.agent_id);
        debug!("group_id={}", self.agent_group_info.group_id);
        debug!("session_id={}", self.session_id);
        debug!("Sending heartbeat to start session");

        debug!("Beginning heartbeat loop");
        ctx.spawn(
            actix::fut::wrap_future::<_, Self>(ctx.address().send(LoopedHeartbeat {}))
                .map(|_, _, _| ()),
        );

        Box::pin(
            actix::fut::wrap_future::<_, Self>(ctx.address().send(Heartbeat {
                session_id: self.session_id.clone(),
            }))
            .map(move |result, _, _| match result {
                Err(err) => {
                    error!("Internal error sending heartbeat message: {}", err);

                    Err(SessionErr {
                        error: Some(Box::new(err)),
                    })
                }
                Ok(Err(err)) => {
                    error!("Error sending heartbeat message: {:?}", err);

                    Err(err)
                }
                Ok(Ok(())) => {
                    info!("Heartbeat was successful.");

                    Ok(())
                }
            }),
        )
    }
}

impl Handler<LoopedHeartbeat> for SessionActor {
    type Result = ResponseActFuture<Self, Result<(), SessionErr>>;

    fn handle(&mut self, _msg: LoopedHeartbeat, ctx: &mut Context<Self>) -> Self::Result {
        ctx.run_later(Duration::new(HEARTBEAT_DURATION_SEC, 0), move |_, ctx| {
            ctx.spawn(
                actix::fut::wrap_future::<_, Self>(ctx.address().send(LoopedHeartbeat {}))
                    .map(|_, _, _| ()),
            );
        });

        Box::pin(
            actix::fut::wrap_future::<_, Self>(Box::pin(ctx.address().send(Heartbeat {
                session_id: self.session_id.to_owned(),
            })))
            .map(|_, _, _| Ok(())),
        )
    }
}

impl Handler<Heartbeat> for SessionActor {
    type Result = ResponseActFuture<Self, Result<(), SessionErr>>;

    fn handle(&mut self, msg: Heartbeat, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling session heartbeat");
        debug!("agent_id={}", self.agent_group_info.agent_id);
        debug!("group_id={}", self.agent_group_info.group_id);
        debug!("session_id={}", msg.session_id);

        let heartbeat = SessionHeartbeat {
            group_id: self.agent_group_info.group_id.to_owned(),
            agent_id: self.agent_group_info.agent_id.to_owned(),
            session_id: msg.session_id,
        };

        let config_addr = self.configurator_addr.clone();

        Box::pin(
            actix::fut::wrap_future::<_, Self>(self.api_addr.send(heartbeat)).then(
                move |result, _actor, _ctx| {
                    actix::fut::wrap_future::<_, Self>(async move {
                        match result {
                            Err(err) => {
                                error!("Internal error sending heartbeat");

                                Err(SessionErr {
                                    error: Some(Box::new(err)),
                                })
                            }
                            Ok(Err(_err)) => {
                                error!("API error sending heartbeat");

                                Err(SessionErr { error: None })
                            }
                            Ok(Ok(session)) => {
                                let result = config_addr
                                    .send(SessionState {
                                        timestamp: Utc::now(),
                                        agent_session_id: session.agent_session_id,
                                        monitors: session.monitors,
                                        heartbeat_due_by: session.heartbeat_due_by,
                                    })
                                    .await;

                                if let Err(err) = result {
                                    error!("Error updating session state: {}", err);
                                    // TODO perform error handling action
                                }

                                Ok(())
                            }
                        }
                    })
                },
            ),
        )
    }
}

impl Actor for SessionActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("SessionActor stopped");
    }
}
