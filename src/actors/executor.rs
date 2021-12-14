use crate::error::Error;
use actix::prelude::*;
use futures::stream::FuturesUnordered;
use openapi_client::models;
use std::collections::HashSet;
use chrono::prelude::*;
use crate::monitoring::Monitoring;

pub struct ExecutorActor<M> {
    monitoring: M,
    busy_monitors: HashSet<String>,
    recipients: Vec<Recipient<StatusMsg>>,
}

impl<M: Monitoring> ExecutorActor<M> {
    pub fn new(monitoring: M, recipients: Vec<Recipient<StatusMsg>>) -> Self {
        Self {
            monitoring,
            busy_monitors: HashSet::new(),
            recipients, 
        }
    }
}

pub struct ExecReport {
    pub monitors_started: Vec<String>,
    pub monitors_ignored: Vec<String>,
}

impl ExecReport {
    pub fn new() -> Self {
        Self {
            monitors_started: vec![],
            monitors_ignored: vec![],
        }
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<ExecReport, Error>")]
pub struct ExecuteBatch {
    pub monitors: Vec<models::Monitor>,
}

impl<M: Send + Unpin + 'static> Actor for ExecutorActor<M> {
    type Context = Context<Self>;

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        Running::Continue // TODO why does this actor stop?!
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("ExecutorActor stopped");
    }
}

impl<M: Send + Unpin + 'static> StreamHandler<(models::Monitor, models::MonitorStatus)> for ExecutorActor<M> {
    fn handle(&mut self, (monitor, status): (models::Monitor, models::MonitorStatus), ctx: &mut Self::Context) {
        debug!("Got monitor status (monitor={}, status={})", 
            monitor.name,
            status.status 
        );
        for r in self.recipients.iter() {
            ctx.spawn(actix::fut::wrap_future(r.send(StatusMsg { 
                status: status.clone(),
                monitor: monitor.clone(),
            })).map(|result, _, _| {
                match result {
                    Ok(_) => {},
                    Err(err) => error!("Error delivering status msg (error_msg={})", err)
                }
            }));
        }
    }
}

impl<M: Monitoring + Send + Unpin + 'static> Handler<ExecuteBatch> for ExecutorActor<M> {
    type Result = Result<ExecReport, Error>;

    fn handle(&mut self, msg: ExecuteBatch, ctx: &mut Context<Self>) -> Self::Result {
        if msg.monitors.is_empty() {
            return Ok(ExecReport {
                monitors_started: vec![],
                monitors_ignored: vec![],
            });
        }

        debug!("Executing batch of {} monitors", msg.monitors.len());

        let monitor_futures = FuturesUnordered::new();
        let mut report = ExecReport::new();
        for monitor in msg.monitors.into_iter() {
            let monitor_name = &monitor.name;
            if !self.busy_monitors.contains(monitor_name) {
                let monitor_copy = monitor.clone();
                let fut = self.monitoring.monitor(&monitor_copy);

                let monitor_name = monitor_copy.name.clone();
                let monitor_type = monitor.type_;
                let status_id = monitor.name;

                report.monitors_started.push(monitor_name.to_owned());
                monitor_futures.push(Box::pin(async move {
                    let timestamp = Utc::now();
                    let status = match fut.await {
                        Ok(s) => s,
                        Err(err) => {
                            models::MonitorStatus {
                                monitor_name,
                                monitor_type,
                                status: models::MonitorStatusIndicator::DOWN,
                                status_id,
                                timestamp,
                                expires_at: timestamp + chrono::Duration::days(1), // TODO
                                expected_result: "Expected to be able to start monitor".to_string(),
                                actual_result: format!("Starting monitor failed: {}", err),
                                description: format!("Monitor of type {}", monitor_copy.type_),
                                session: None,
                                log: Vec::new() 
                            }
                        },
                    };
                    (monitor_copy, status)
                }));
            } else {
                report.monitors_ignored.push(monitor_name.to_owned());
            }
        }

        ctx.add_stream(monitor_futures);

        Ok(report)
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct StatusMsg {
    pub monitor: models::Monitor,
    pub status: models::MonitorStatus
}
