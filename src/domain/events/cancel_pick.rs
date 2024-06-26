use std::sync::Arc;

use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub channel: String,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Empty,
    NotFound,
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<(), Error> {
    let channel = repo
        .find_channel_by_name(req.channel.clone())
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;
    let event = repo
        .find_event(req.event.clone(), channel.id)
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    if event.participants.len() == 0 {
        return Err(Error::Empty);
    }

    repo.clone()
        .rev_pick(event.id, channel.id)
        .await
        .map_err(|error| match error {
            UpdateError::NotFound => Error::NotFound,
            UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
        })?;

    Ok(())
}
