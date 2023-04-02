use std::sync::Arc;

use crate::repository::{errors::CountError, event};

pub struct Request {
    channel: String,
}

pub struct Response {
    pub count: u32,
}

impl From<u32> for Response {
    fn from(count: u32) -> Self {
        Self { count }
    }
}

pub enum Error {
    Unknown,
}

impl From<CountError> for Error {
    fn from(value: CountError) -> Self {
        match value {
            CountError::Unknown => Self::Unknown,
        }
    }
}

pub async fn execute(
    event_repo: Arc<dyn event::Repository>,
    req: Request,
) -> Result<Response, Error> {
    let channel = match event_repo.clone().find_channel_by_name(req.channel).await {
        Ok(channel) => channel,
        _ => return Ok(Response::from(0)),
    };
    Ok(Response::from(event_repo.count_events(channel.id).await?))
}
