use actix::prelude::*;
use crate::actors::*;
use crate::error::Error;
use openapi_client::models;
use std::time;
use std::collections::HashSet;

pub struct UploaderActor {
    api_addr: Addr<ApiActor>,
    buffer: Vec<models::MonitorStatus>,
    last_upload_started: Option<time::Instant>,
    interval: Option<SpawnHandle>,
}

impl UploaderActor {
    pub fn new(api_addr: Addr<ApiActor>) -> Self {
        Self {
            api_addr,
            buffer: vec![],
            //offset: 0,
            last_upload_started: None,
            interval: None
        }
    }

    fn ensure_upload(&mut self, ctx: &mut <Self as Actor>::Context) {
        let ten_seconds: time::Duration = time::Duration::new(10, 0);
        if self.last_upload_started.is_none() {
            // no uploading has been performed, do it now
            self.perform_upload(ctx);
        } else if let Some(last_started) = self.last_upload_started {
            debug!("Holding off upload");
            // it has been ten seconds since the last upload, do it now
            if time::Instant::now() - last_started < ten_seconds {
                self.perform_upload(ctx);
            }
        }

        if self.interval.is_none() {
            self.interval = Some(ctx.run_interval(ten_seconds, |this, ctx| {
                this.perform_upload(ctx);
            }));
        }
    }

    fn perform_upload(&mut self, ctx: &mut <Self as Actor>::Context) { 
        // if empty, STOP
        if self.buffer.is_empty() {
            return;
        }
        debug!("Performing upload");
        let mut already_seen = HashSet::new();

        // sort the results by timestamp
        let mut buffer = self.buffer.clone();
        buffer.sort_by(|s1, s2| s2.timestamp.cmp(&s1.timestamp));

        // filter out the most recent results
        let statuses: Vec<_> = buffer.into_iter()
            .filter(|status| {
                if already_seen.contains(&status.monitor_name) {
                    false
                } else {
                    already_seen.insert(status.monitor_name.clone());
                    true
                }
            })
            .collect();

        self.buffer = vec![];

        // send message to API actor
        debug!("Uploading statuses (statuses_len={})", statuses.len());
        let request = self.api_addr.send(PostStatuses { statuses: statuses.clone() });

        ctx.spawn(
            actix::fut::wrap_future::<_, Self>(request).map(move |result, this, _| {
                match result {
                    Ok(_) => {
                        debug!("Upload was successful");
                    },
                    Err(err) => {
                        error!("Error while uploading: {}", err);
                        debug!("Putting these statuses back on the buffer");
                        for status in statuses.into_iter() {
                            this.buffer.insert(0, status);
                        }
                    }
                }
            })
        );
    }
}

impl Actor for UploaderActor {
    type Context = Context<Self>;
}

impl Handler<StatusMsg> for UploaderActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: StatusMsg, ctx: &mut Self::Context) -> Self::Result {
        debug!("Status received (monitor_name={})", msg.status.monitor_name);
        self.buffer.push(msg.status);
        debug!("Message added to buffer (buffer_len={})", self.buffer.len());
        self.ensure_upload(ctx);
        Ok(())
    }
}
