use actix::Handler;
use crate::error::Error;
use crate::api::ReadApi;

#[derive(Clone, Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct MonitorSourceActor {
    api: Box<dyn ReadApi>,
}
