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
    Ok(Response::from(event_repo.count_events(req.channel).await?))
}
