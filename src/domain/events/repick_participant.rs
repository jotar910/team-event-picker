use std::sync::Arc;

use crate::domain::entities::User;
use crate::domain::events::pick_participant;
use crate::domain::helpers;
use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub channel: String,
}

impl From<Request> for pick_participant::Request {
    fn from(value: Request) -> Self {
        Self {
            event: value.event,
            channel: value.channel,
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub id: u32,
    pub name: String,
}

impl From<User> for Response {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<Response> for User {
    fn from(value: Response) -> User {
        User {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Empty,
    NotFound,
    Unknown,
}

impl From<pick_participant::Error> for Error {
    fn from(value: pick_participant::Error) -> Self {
        match value {
            pick_participant::Error::Empty => Self::Empty,
            pick_participant::Error::NotFound => Self::NotFound,
            pick_participant::Error::Unknown => Self::Unknown,
        }
    }
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

    let (pick, participant) = helpers::repick(&event);
    repo.save_pick(pick).await.map_err(|error| match error {
        UpdateError::NotFound => Error::NotFound,
        UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
    })?;

    Ok(repo
        .find_user(participant)
        .await
        .map_err(|error| match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        })?
        .into())
}
