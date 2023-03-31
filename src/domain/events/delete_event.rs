use std::sync::Arc;

use serde::Serialize;

use crate::repository::errors::{DeleteError, FindError};
use crate::repository::event::Repository;

#[derive(Debug, PartialEq)]
pub enum Error {
    NotFound,
    Unknown,
}
pub struct Request {
    pub id: u32,
    pub channel: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Response {
    pub id: u32,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let channel = repo
        .find_channel_by_name(req.channel.clone())
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    let event = match repo.delete_event(req.id, channel.id).await {
        Err(err) => {
            return match err {
                DeleteError::NotFound => Err(Error::NotFound),
                DeleteError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(event) => event,
    };
    Ok(Response { id: event.id })
}
