use crate::actors::*;
use crate::error::Error;
use crate::futures::FutureExt;
use actix::clock::delay_until;
use actix::clock::Instant;
use log::*;
use openapi_client::models;
use std::time;

pub struct SchedulerActor {
    monitors: Vec<models::Monitor>,
    schedule_states: Vec<ScheduleState>,
    interval: Option<(time::Duration, SpawnHandle)>,
    last_wakeup: Option<time::Instant>,
    recipient: Recipient<ExecuteBatch>,
}

impl Actor for SchedulerActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("SchedulerActor stopped");
    }
}

impl SchedulerActor {
    pub fn new(recipient: Recipient<ExecuteBatch>) -> Self {
        Self {
            monitors: vec![],
            schedule_states: vec![],
            interval: None,
            last_wakeup: None,
            recipient
        }
    }

    fn do_scheduling(
        &mut self,
        ctx: &mut <Self as Actor>::Context,
    ) -> RecipientRequest<ExecuteBatch> {
        let now = time::Instant::now();
        let mut monitors_due_now = Vec::new();

        for schedule_state in self.schedule_states.iter() {
            let monitor = match self.get_monitor(&schedule_state.monitor_id) {
                Some(m) => m,
                None => {
                    // TODO remove the monitor
                    continue;
                }
            };
            let period_duration = get_duration(monitor);
            let due_now = if let Some(last_scheduled) = schedule_state.last_scheduled {
                last_scheduled + period_duration <= now
            } else {
                true
            };

            // between last_wakeup and interval, which monitors will be due
            if let (Some(last_scheduled), Some((duration, _)), Some(last_wakeup)) = (
                schedule_state.last_scheduled,
                self.interval,
                self.last_wakeup,
            ) {
                // TODO the following is not bounded
                // only one should be scheduled OR scheduling should reach a steady state
                let due_next_at = last_scheduled + period_duration;

                let addr = ctx.address().clone();

                if due_next_at <= last_wakeup + duration {
                    ctx.spawn(
                        actix::fut::wrap_future::<_, Self>(async move {
                            delay_until(Instant::from_std(due_next_at)).await;

                            let result = addr
                                .send(ScheduleTimeout {
                                    timestamp: time::Instant::now(),
                                    wait_duration: duration,
                                })
                                .await;

                            if let Err(err) = result {
                                error!("Error during sending ScheduleTimeout: {}", err);
                            }
                        })
                        .map(|_, _, _| ()),
                    );
                }
            }

            if due_now {
                debug!("Monitor {} due to be run", if monitor.name.is_empty() { 
                    format!("{:?}", monitor.id)
                } else {
                    monitor.name.to_owned()
                });
                monitors_due_now.push(monitor.clone());
            }
        }

        let affected_monitors: Vec<_> = monitors_due_now.iter().map(|m| m.id.clone()).collect();

        for monitor_id_option in affected_monitors {
            if let Some(monitor_id) = monitor_id_option {
                if let Some(ref mut schedule_state) = self.get_schedule_state(&monitor_id) {
                    schedule_state.count += 1;
                    schedule_state.last_scheduled = Some(now);
                    debug!(
                        "New schedule count for monitor (monitor_id={}, count={})",
                        monitor_id, schedule_state.count
                    );
                }
            }
        }

        self.recipient.send(ExecuteBatch {
            monitors: monitors_due_now,
        })
    }

    fn set_interval(&mut self, ctx: &mut <Self as Actor>::Context) {
        debug!("Setting up interval");

        // TODO clean this up
        const ONE_MINUTE: u64 = 60;
        let mut shortest_duration = time::Duration::new(ONE_MINUTE, 0);
        for monitor in self.monitors.iter() {
            let duration = get_duration(monitor);
            if duration < shortest_duration {
                shortest_duration = duration;
            }
        }

        if let Some((duration, existing_interval)) = self.interval {
            if duration == shortest_duration {
                debug!(
                    "No need to update interval. Keeping at {}s",
                    duration.as_secs()
                );
                return;
            }
            ctx.cancel_future(existing_interval);
        }

        debug!("Interval set at {}s", shortest_duration.as_secs());

        self.interval = Some((
            shortest_duration,
            ctx.run_interval(shortest_duration, move |this, mut ctx| {
                debug!("Interval wakeup triggered. Scheduling timeout");
                let schedule_future = this.do_scheduling(&mut ctx);
                ctx.spawn(
                    actix::fut::wrap_future::<_, Self>(schedule_future) //ctx.address().send(schedule_timeout))
                        .map(|_, _, _| ()),
                );
            }),
        ));
    }

    fn get_monitor(&self, id: &str) -> Option<&models::Monitor> {
        for m in self.monitors.iter() {
            if m.id == Some(id.to_string()) {
                return Some(m);
            }
        }

        None
    }

    fn get_schedule_state(&mut self, monitor_id: &str) -> Option<&mut ScheduleState> {
        for s in self.schedule_states.iter_mut() {
            if s.monitor_id == *monitor_id {
                return Some(s);
            }
        }

        None
    }

    fn consume_monitors(&mut self, monitors: Vec<models::Monitor>) {
        debug!("Updating with {} monitors", monitors.len());

        self.monitors = monitors;

        for monitor in self.monitors.iter() {
            let monitor_id = match &monitor.id {
                &Some(ref monitor_id) => monitor_id.to_owned(),
                &None => {
                    error!("Found monitor ID (name={})", monitor.name);
                    continue;
                }
            };

            let mut has_entry = false;

            for schedule_state in self.schedule_states.iter() {
                if schedule_state.monitor_id == monitor_id {
                    has_entry = true;
                }
            }

            if !has_entry {
                debug!("Adding new schedule record (monitor_id={})", monitor_id);
                self.schedule_states.push(ScheduleState {
                    monitor_id,
                    last_scheduled: None,
                    count: 0,
                });
            }
        }
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct MonitorUpdate {
    pub monitors: Vec<models::Monitor>,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct ScheduleTimeout {
    pub timestamp: time::Instant,
    pub wait_duration: time::Duration,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<Status, ()>")]
pub struct StatusRequest;

#[derive(Clone, Debug)]
pub struct Status {
    schedule_states: Vec<ScheduleState>,
}

impl Handler<StatusRequest> for SchedulerActor {
    type Result = Result<Status, ()>;

    fn handle(&mut self, _: StatusRequest, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Status request {:?}", self.schedule_states);
        Ok(Status {
            schedule_states: self.schedule_states.clone(),
        })
    }
}

impl Handler<MonitorUpdate> for SchedulerActor {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, msg: MonitorUpdate, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling monitor update");
        self.consume_monitors(msg.monitors);
        self.set_interval(ctx);

        Box::pin(actix::fut::wrap_future::<_, Self>(self.do_scheduling(ctx)).map(|_, _, _| Ok(())))
    }
}

impl Handler<ScheduleTimeout> for SchedulerActor {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, _msg: ScheduleTimeout, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling ScheduleTimeout");
        self.last_wakeup = Some(time::Instant::now());
        // a timeout has expired! check the schedule log
        let scheduling_fut = self.do_scheduling(ctx).map(|_| Ok(()));

        Box::pin(actix::fut::wrap_future::<_, Self>(scheduling_fut))
    }
}

#[derive(Clone, Debug)]
struct ScheduleState {
    monitor_id: String,
    last_scheduled: Option<time::Instant>,
    count: u64,
}

fn get_duration(monitor: &models::Monitor) -> time::Duration {
    let mut period_millis = to_milliseconds(&monitor.period);
    if period_millis <= 0 {
        warn!("Unusual period for monitor (id={:?})", monitor.id);
        period_millis = 1000;
    }
    time::Duration::from_millis(period_millis as u64)
}

fn to_milliseconds(time: &str) -> i32 {
    const BASE_10: u32 = 10;
    let parts = time.split_whitespace();
    for part in parts {
        let mut unit = 0;

        for c in part.chars() {
            if c.is_digit(BASE_10) {
                unit = unit * 10;
                let unit_string = format!("{}", c);
                unit += unit_string.parse::<i32>().unwrap();
            } else {
                return unit
                    * match c {
                        's' => SECOND,
                        'm' => MINUTE,
                        'h' => HOUR,
                        'd' => DAY,
                        _ => 0, // unreachable hopefully
                    };
            }
        }
    }

    0
}

pub const MILLI: i32 = 1;
pub const SECOND: i32 = 1000 * MILLI;
pub const MINUTE: i32 = 60 * SECOND;
pub const HOUR: i32 = MINUTE * 60;
pub const DAY: i32 = HOUR * 24;
