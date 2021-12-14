use crate::actors::TimerActor;
use crate::actors::*;
use crate::error::Error;
use actix::clock::Instant;
use log::*;
use openapi_client::models;
use std::collections::HashMap;
use std::time;

pub struct SchedulerActor {
    monitors: HashMap<String, MonitorContainer>,
    recipients: Vec<Recipient<ExecuteBatch>>,
    timer: Addr<TimerActor>,
}

struct MonitorContainer {
    uid: String,
    monitor: models::Monitor,
}

impl Actor for SchedulerActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("SchedulerActor stopped");
    }
}

impl SchedulerActor {
    pub fn new(recipients: Vec<Recipient<ExecuteBatch>>, timer: Addr<TimerActor>) -> Self {
        Self {
            monitors: HashMap::new(),
            recipients,
            timer,
        }
    }
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct MonitorUpdate {
    /// Uniquely identifies this set of monitors
    pub source_id: String,
    pub monitor: models::Monitor,
}

impl Handler<MonitorUpdate> for SchedulerActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: MonitorUpdate, ctx: &mut Context<Self>) -> Self::Result {
        let uid = format!("https://api.schnooty.com/monitors/{}", msg.monitor.name);

        debug!("Handling monitor update");

        self.monitors.insert(
            uid.clone(),
            MonitorContainer {
                uid: uid.clone(),
                monitor: msg.monitor.clone(),
            },
        );

        let recipient = ctx.address().recipient();
        let period = get_duration(&msg.monitor);

        self.timer.do_send(TimerSpec {
            uid,
            recipient,
            period,
        });

        Ok(())
    }
}

impl Handler<Timeout> for SchedulerActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: Timeout, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Waking up scheduler");

        match self
            .monitors
            .iter()
            .filter(|(ref uid, _)| *uid == &msg.uid)
            .next()
        {
            Some((_, ref container)) => {
                let message = ExecuteBatch {
                    monitors: vec![container.monitor.clone()],
                };
                for recv in self.recipients.iter() {
                    if !recv.do_send(message.clone()).is_ok() {
                        // do nothing
                    }
                }
                Ok(())
            }
            None => {
                debug!("Got timeout for {} but this monitor not found", msg.uid);
                Ok(())
            }
        }
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
    monitor_name: String,
}

pub const MILLI: i32 = 1;
pub const SECOND: i32 = 1000 * MILLI;
pub const MINUTE: i32 = 60 * SECOND;
pub const HOUR: i32 = MINUTE * 60;
pub const DAY: i32 = HOUR * 24;
