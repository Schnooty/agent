use crate::actors::*;
use crate::error::Error;
use std::collections::HashMap;
use std::time::Duration;

pub struct TimerActor {
    schedule: HashMap<String, Receiver>,
}

impl TimerActor {
    pub fn new() -> Self {
        Self {
            schedule: HashMap::new(),
        }
    }
}

impl Actor for TimerActor {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("TimerActor stopped");
    }
}

struct Receiver {
    #[allow(dead_code)]
    spec: TimerSpec,
    interval: SpawnHandle,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct TimerSpec {
    pub uid: String,
    pub recipient: Recipient<Timeout>,
    pub period: Duration,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct Timeout {
    pub uid: String,
}

impl Handler<TimerSpec> for TimerActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: TimerSpec, ctx: &mut Context<Self>) -> Self::Result {
        // if not already scheduled
        debug!("Setting up timer for {}", msg.uid);
        match self.schedule.remove(&msg.uid) {
            None => {
                if !msg
                    .recipient
                    .do_send(Timeout {
                        uid: msg.uid.clone(),
                    })
                    .is_ok()
                {
                    error!(
                        "Failed to send timeout message while removing spec for {}",
                        msg.uid
                    );
                    return Err(Error::new(format!(
                        "Failed to set timer spec for {}",
                        msg.uid
                    )));
                }
            }
            Some(s) => {
                debug!("Cancelling existing timer");
                ctx.cancel_future(s.interval);
            }
        }

        let rec = msg.recipient.clone();
        let uid_int = msg.uid.clone();

        let interval = ctx.run_interval(msg.period, move |_, _| {
            if let Err(err) = rec.do_send(Timeout {
                uid: uid_int.clone(),
            }) {
                error!("Error sending timeout to {}: {}", uid_int, err);
            }
        });

        self.schedule.insert(
            msg.uid.clone(),
            Receiver {
                spec: msg,
                interval,
            },
        );

        debug!("Successfully set up timer");

        Ok(())
    }
}
