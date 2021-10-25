use crate::actors::ApiActor;
use crate::actors::*;
use crate::error::Error;
use crate::api::HttpApi;
use actix::ResponseActFuture;
use chrono::prelude::*;

pub struct ConfiguratorActor {
    api_addr: Option<Addr<ApiActor<HttpApi>>>,
    monitor_file_addr: Option<Addr<FileActor>>,
    scheduler_addr: Addr<SchedulerActor>,
    alert_recipients: Vec<Recipient<AlertUpdate>>,
}

impl ConfiguratorActor {
    pub fn new(
        api_addr: Option<Addr<ApiActor<HttpApi>>>,
        scheduler_addr: Addr<SchedulerActor>,
        alert_recipients: Vec<Recipient<AlertUpdate>>,
        monitor_file_addr: Option<Addr<FileActor>>
    ) -> Self {
        Self {
            api_addr,
            scheduler_addr,
            alert_recipients,
            monitor_file_addr
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
    //pub monitors: Vec<String>,
    pub heartbeat_due_by: DateTime<Utc>,
}

impl Handler<SessionState> for ConfiguratorActor {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, _: SessionState, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling session state");
        let api_addr = self.api_addr.clone();
        let monitor_file_addr = self.monitor_file_addr.clone();
        let scheduler_addr = self.scheduler_addr.clone();
        let alert_recipients = self.alert_recipients.clone();

        Box::pin(actix::fut::wrap_future(async move {
            let mut monitors = Vec::new();

            if let Some(ref api_addr) = &api_addr {
                match api_addr.send(GetMonitors).await {
                    Ok(Ok(mut m)) => monitors.append(&mut m),
                    Ok(Err(err)) => { 
                        debug!("Error getting monitors: {}", err);
                        return Err(err);
                    },
                    Err(err) => { 
                        debug!("Error getting monitors: {}", err);
                        return Err(Error::from(err));
                    }
                };
            }

            if let Some(monitor_file_addr) = monitor_file_addr {
                match monitor_file_addr.send(GetMonitors).await {
                    Ok(Ok(mut m)) => monitors.append(&mut m),
                    Ok(Err(err)) => { 
                        error!("Error loading monitors from file: {}", err);
                        return Err(err);
                    }
                    Err(err) => { 
                        error!("Error loading monitors from file: {}", err);
                        return Err(Error::from(err));
                    }
                }
            };

            let mut alerts = Vec::new();

            if let Some(ref api_addr) = &api_addr {
                match api_addr.send(GetAlerts).await {
                    Ok(Ok(mut a)) => alerts.append(&mut a),
                    Ok(Err(err)) => return Err(err),
                    Err(err) => return Err(Error::from(err)),
                };
            }

            for alert_recipient in alert_recipients {
                match alert_recipient.do_send(AlertUpdate { alerts: alerts.clone() }) {
                    Err(err) => error!("Error sending alert update from configurator (error_msg={})", err),
                    Ok(_) => ()
                }
            }

            // now filter out the monitors that we will use

            match scheduler_addr
                .send(MonitorUpdate {
                    monitors
                })
                .await
            {
                Ok(_) => Ok(()),
                Err(err) => Err(Error::from(err)),
            }
        }))
    }
}
