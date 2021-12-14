use crate::actors::*;
use crate::error::Error;
use crate::config::Config;

pub struct ConfiguratorActor {
    monitor_recipients: Vec<Recipient<MonitorUpdate>>,
    alert_recipients: Vec<Recipient<AlertUpdate>>,
}

impl ConfiguratorActor {
    pub fn new(
        monitor_recipients: Vec<Recipient<MonitorUpdate>>,
        alert_recipients: Vec<Recipient<AlertUpdate>>,
    ) -> Self {
        Self {
            monitor_recipients,
            alert_recipients,
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
pub struct CurrentConfig {
    pub config: Config
}

impl Handler<CurrentConfig> for ConfiguratorActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, config_msg: CurrentConfig, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling latest config");

        // create a unique identifier for these fixed monitors
        let monitors_uid = format!("config://monitors"); // yes it is a URI
        let alerts_uid = format!("config://alerts"); // yes it is a URI

        for monitor_recipient in &self.monitor_recipients {
            for monitor in config_msg.config.monitors.iter() {
                // build the monitor config message
                let monitor_update = MonitorUpdate {
                    source_id: monitors_uid.clone(),
                    monitor: monitor.clone() 
                };
                if let Err(_) = monitor_recipient.do_send(monitor_update.clone()) {
                    error!("There was an error delivering monitors");
                }
            }
        }

        let alert_update = AlertUpdate {
            uid: alerts_uid,
            alerts: config_msg.config.alerts
        };

        for alert_recipient in &self.alert_recipients {
            if let Err(_) = alert_recipient.do_send(alert_update.clone()) {
                error!("There was an error delivering alerts");
            }
        }

       
        Ok(())
    }
}
