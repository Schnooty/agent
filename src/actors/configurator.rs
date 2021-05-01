use crate::actors::ApiActor;
use crate::actors::*;
use crate::error::Error;
use crate::api::HttpApi;
use actix::ResponseActFuture;
use chrono::prelude::*;
use openapi_client::models;

pub struct ConfiguratorActor {
    api_addr: Addr<ApiActor<HttpApi>>,
    scheduler_addr: Addr<SchedulerActor>,
    alert_recipients: Vec<Recipient<AlertUpdate>>
}

impl ConfiguratorActor {
    pub fn new(api_addr: Addr<ApiActor<HttpApi>>, scheduler_addr: Addr<SchedulerActor>, alert_recipients: Vec<Recipient<AlertUpdate>>) -> Self {
        Self {
            api_addr,
            scheduler_addr,
            alert_recipients
        }
    }
}

impl Actor for ConfiguratorActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("ConfiguratorActor stopped");
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct SessionState {
    pub timestamp: DateTime<Utc>,
    pub agent_session_id: String,
    pub monitors: Vec<String>,
    pub heartbeat_due_by: DateTime<Utc>,
}

impl Handler<SessionState> for ConfiguratorActor {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, msg: SessionState, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling session state");
        let api_addr = self.api_addr.clone();
        let scheduler_addr = self.scheduler_addr.clone();
        let alert_recipients = self.alert_recipients.clone();

        Box::pin(actix::fut::wrap_future(async move {
            let monitors = match api_addr.send(GetMonitors).await {
                Ok(Ok(m)) => m,
                Ok(Err(err)) => return Err(err),
                Err(err) => return Err(Error::from(err)),
            };

            let alerts = match api_addr.send(GetAlerts).await {
                Ok(Ok(m)) => m,
                Ok(Err(err)) => return Err(err),
                Err(err) => return Err(Error::from(err)),
            };

            for alert_recipient in alert_recipients {
                match alert_recipient.do_send(AlertUpdate { alerts: alerts.clone() }) {
                    Err(err) => error!("Error sending alert update from configurator (error_msg={})", err),
                    Ok(_) => ()
                }
            }

            // now filter out the monitors that we will use

            let our_monitors: Vec<models::Monitor> = monitors
                .into_iter()
                .filter(|m| {
                    if let Some(monitor_id) = &m.id {
                        msg.monitors.contains(&monitor_id)
                    } else {
                        false
                    }
                })
                .collect();

            match scheduler_addr
                .send(MonitorUpdate {
                    monitors: our_monitors,
                })
                .await
            {
                Ok(_) => Ok(()),
                Err(err) => Err(Error::from(err)),
            }
        }))
    }
}
