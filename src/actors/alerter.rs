use crate::actors::*;
use crate::alerts::*;
use crate::error::Error;
use crate::openapi_client::models;
use chrono::offset::Utc;
use chrono::DateTime;
use hostname::get as get_hostname;
use std::collections::HashMap;
use std::env;
use sysinfo::SystemExt;

#[allow(dead_code)]
pub struct AlerterActor {
    status_buffer: Vec<(models::Monitor, models::MonitorStatus)>,
    statuses: HashMap<String, MonitorState>,
    api: Box<dyn AlertApi>,
    alerts: Vec<models::Alert>,
}

impl AlerterActor {
    pub fn new<A: AlertApi + 'static>(api: A) -> Self {
        Self {
            status_buffer: vec![],
            statuses: HashMap::new(),
            api: Box::new(api),
            alerts: vec![],
        }
    }
}

impl AlerterActor {
    fn process_state_change(&mut self, ctx: &mut <Self as Actor>::Context) {
        self.status_buffer
            .sort_by(|s1, s2| s1.1.timestamp.cmp(&s2.1.timestamp));

        for (monitor, status) in self.status_buffer.iter() {
            if !self.statuses.contains_key(&status.status_id) {
                self.statuses
                    .insert(status.status_id.clone(), MonitorState::new(monitor, status));
            }
            match self.statuses.get_mut(&status.monitor_name) {
                Some(ref mut current_state) => {
                    let is_down = status.status == models::MonitorStatusIndicator::DOWN;

                    let state_changed = match (
                        current_state.last_timestamp <= status.timestamp,
                        current_state.last_status.clone(),
                        status,
                    ) {
                        (true, previous, current) => previous.status != current.status,
                        _ => false,
                    };

                    if state_changed || (is_down && current_state.is_new) {
                        info!("Detected monitor state change (monitor_name={}, previous_statues={}, current_state={})",
                            status.monitor_name, current_state.last_status.status, status.status);

                        current_state.is_new = false;
                        current_state.last_status = status.clone();

                        perform_state_change_action(
                            &mut *self.api,
                            &current_state,
                            ctx,
                            &self.alerts,
                        );
                    }
                }
                None => unreachable!(
                    "Could not load status that was just inserted (monitor_name={})",
                    status.monitor_name
                ),
            }
        }

        self.status_buffer = vec![];
    }
}

#[allow(dead_code)]
fn perform_state_change_action(
    api: &mut dyn AlertApi,
    state: &MonitorState,
    ctx: &mut <AlerterActor as Actor>::Context,
    alerts: &[models::Alert],
) {
    let status = &state.last_status;
    let payload = AlertPayload {
        monitor_name: status.monitor_name.to_owned(),
        //monitor_name: state.monitor.name.to_owned(),
        status: status.clone(),
        node_info: get_node_info(),
    };
    for alert in alerts.iter() {
        let alert_future = match alert.type_.as_ref() {
            "email" => {
                info!("Sending email for alert (id={:?})", alert.id);
                api.send_email(
                    &models::EmailAlertBody {
                        from: alert.body.from.clone(),
                        recipients: alert.body.recipients.clone(),
                        host: alert.body.host.clone(),
                        port: alert.body.port,
                        tls_mode: alert.body.tls_mode,
                        username: alert.body.username.clone(),
                        password: alert.body.password.clone(),
                    },
                    &payload,
                )
            }
            "msTeamsMessage" => api.send_msteams_msg(
                &models::MsTeamsAlertBody {
                    url: alert.body.url.to_owned(),
                },
                &payload,
            ),
            "webhook" => api.send_webhook(
                &models::WebhookAlertBody {
                    headers: alert.body.headers.clone(),
                    url: alert.body.url.to_owned(),
                },
                &payload,
            ),
            "log" => {
                return; // TODO
            }
            _ => {
                error!("Severe error. Unkown alert type (type={})", alert.type_);
                return;
            }
        };

        ctx.spawn(actix::fut::wrap_future::<_, AlerterActor>(alert_future).map(|_, _, _| ()));
    }
}

impl Actor for AlerterActor {
    type Context = Context<Self>;
}

impl Handler<StatusMsg> for AlerterActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: StatusMsg, ctx: &mut Self::Context) -> Self::Result {
        debug!(
            "Received status update(s) (monitor_name={}, status={}",
            msg.monitor.name, msg.status.status
        );
        self.status_buffer.push((msg.monitor, msg.status));
        self.process_state_change(ctx);

        Ok(())
    }
}

impl MonitorState {
    fn new(_monitor: &models::Monitor, last_status: &models::MonitorStatus) -> Self {
        Self {
            last_timestamp: Utc::now(), // TODO
            last_status: last_status.clone(),
            //monitor: monitor.clone(),
            is_new: true,
        }
    }
}

struct MonitorState {
    last_timestamp: DateTime<Utc>,
    last_status: models::MonitorStatus,
    //monitor: models::Monitor,
    is_new: bool,
}

fn get_node_info() -> NodeInfo {
    const HOSTNAME_UNAVAILABLE: &str = "Hostname unavailable";
    //const CPU_INFO_UNAVAILABLE: &'static str = "CPU info unavailable";

    let cpu = format!(
        "{} logical cores, {} physical cores",
        num_cpus::get(),
        num_cpus::get_physical()
    );

    let mut system: sysinfo::System = sysinfo::SystemExt::new();
    system.refresh_memory();

    let hostname = match get_hostname().map(|h| h.into_string()) {
        Ok(Ok(hostname)) => hostname,
        _ => HOSTNAME_UNAVAILABLE.to_string(),
    };

    let memory_used = system.get_used_memory();
    let memory_total = system.get_total_memory();

    NodeInfo {
        hostname,
        platform: env::consts::OS.to_string(),
        cpu,
        ram: format!(
            "{} KB used of {} total ({:.2} %)",
            memory_used,
            memory_total,
            100_f64 * (memory_used as f64) / (memory_total as f64)
        ),
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct AlertUpdate {
    pub uid: String,
    pub alerts: Vec<models::Alert>,
}

impl Handler<AlertUpdate> for AlerterActor {
    type Result = ();

    fn handle(&mut self, msg: AlertUpdate, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling alerts");
        self.alerts = msg.alerts;
    }
}
