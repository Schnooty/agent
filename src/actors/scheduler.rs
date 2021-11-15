use crate::actors::*;
use crate::error::Error;
use actix::clock::Instant;
use log::*;
use openapi_client::models;
use std::time;
use core::time::Duration;

pub struct SchedulerActor {
    monitors: Vec<models::Monitor>,
    started_at: Instant,
    log: Vec<ScheduleEvent>,
    recipients: Vec<Recipient<ExecuteBatch>>,
    schedule_interval: Option<SpawnHandle>,
    schedule_events: Vec<ScheduleEvent>
}

impl Actor for SchedulerActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("SchedulerActor stopped");
    }
}

impl SchedulerActor {
    pub fn new(recipients: Vec<Recipient<ExecuteBatch>>) -> Self {
        Self {
            monitors: vec![],
            started_at: Instant::now(),
            log: Vec::new(),
            recipients,
            schedule_interval: None,
            schedule_events: Vec::new()
        }
    }

}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct MonitorUpdate {
    /// Uniquely identifies this set of monitors
    pub uid: String,
    pub monitors: Vec<models::Monitor>,
}

impl Handler<MonitorUpdate> for SchedulerActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: MonitorUpdate, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling monitor update");

        const NANO_MILLI_RATIO: u32 = 1000 * 1000;

        let mut intervals: Vec<u32> = msg.monitors
            .iter()
            .map(|m| to_milliseconds(&m.period).abs() as u32)
            .collect();

        intervals.sort();

        self.monitors = msg.monitors;

        let mut new_spawn = if intervals.len() > 0 {
            let period_milliseconds = intervals[0] * NANO_MILLI_RATIO;

            let spawn_process = move |_: &mut SchedulerActor, ctx: &mut Context<Self>| {
                debug!("Schedule timeout.");
                let address = ctx.address();
                address.do_send(ScheduleTimeout { period_milliseconds });
            };
            
            ctx.address().do_send(ScheduleTimeout { period_milliseconds });

            Some(ctx.run_interval(Duration::new(0, period_milliseconds), spawn_process))
        } else {
            None
        };

        std::mem::swap(&mut new_spawn, &mut self.schedule_interval);


        Ok(())
    }
}


#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct ScheduleTimeout {
    period_milliseconds: u32
}

impl Handler<ScheduleTimeout> for SchedulerActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, _msg: ScheduleTimeout, _ctx: &mut Context<Self>) -> Self::Result {
        let now = Instant::now();

        debug!("Waking up scheduler");

        let next: Vec<_> = self.monitors.iter()
            .map(|monitor| match self.schedule_events.iter()
                    .filter(|s| s.monitor_name == monitor.name)
                    .next() {
                    None => (monitor, Duration::new(u64::max_value(), 0), get_duration(&monitor)),
                    Some(ScheduleEvent { 
                        monitor_name: _,
                        timestamp
                    }) => (monitor, now - *timestamp, get_duration(&monitor))
                }
            )
            .filter(|(_, elapsed_time, period)| elapsed_time >= period)
            .map(|(monitor, _, _)| (monitor.clone(), ScheduleEvent { timestamp: now, monitor_name: monitor.name.clone() }))
            .collect();

        let message = ExecuteBatch {
            monitors: next.iter().map(|(m, _)| m.clone()).collect()
        };

        debug!("Scheduling {} monitor(s)", message.monitors.len());

        let mut events: Vec<ScheduleEvent> = next.iter().cloned().map(|(_, s)| s).collect();
        events.append(&mut self.schedule_events);
        self.schedule_events = events;

        for recipient in self.recipients.iter() {
            if let Err(err) = recipient.do_send(message.clone()) {
                error!("Error sending monitor batch: {}", err);
            }
        }

        self.schedule_events.dedup_by_key(|m| m.monitor_name.clone());

        Ok(())
    }
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

#[derive(Clone)]
struct ScheduleEvent {
    timestamp: Instant,
    monitor_name: String
}

pub const MILLI: i32 = 1;
pub const SECOND: i32 = 1000 * MILLI;
pub const MINUTE: i32 = 60 * SECOND;
pub const HOUR: i32 = MINUTE * 60;
pub const DAY: i32 = HOUR * 24;
