use std::sync::Arc;

use crate::domain::entities::Participant;
use crate::domain::events::pick_participant;
use crate::domain::helpers::participant::{last_picked, pick_new, replace_participant};
use crate::helpers::date::Date;
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
    pub name: String,
}

impl From<Participant> for Response {
    fn from(value: Participant) -> Self {
        Self { name: value.user }
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
    let mut event = repo
        .find_event(req.event.clone(), req.channel.clone())
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    let participants = event.participants;

    let cur_pick = last_picked(&participants);
    if let None = cur_pick {
        return Err(Error::Empty);
    }
    let cur_pick = cur_pick.unwrap();

    let new_pick = match pick_new(&participants) {
        None => return Ok(cur_pick.clone().into()),
        Some(participant) => participant,
    };
    event.participants = replace_participant(
        participants.clone(),
        Participant {
            picked: true,
            picked_at: Some(Date::now().timestamp()),
            ..new_pick.clone()
        },
    );
    event.participants = replace_participant(
        event.participants,
        Participant {
            picked: false,
            picked_at: None,
            ..cur_pick.clone()
        },
    );
    repo.update_event(event).await.map_err(|error| {
        return match error {
            UpdateError::NotFound => Error::NotFound,
            UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
        };
    })?;

    Ok(new_pick.clone().into())
}
