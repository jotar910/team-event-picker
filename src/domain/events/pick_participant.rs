use std::sync::Arc;

use crate::domain::entities::Participant;
use crate::domain::helpers::participant::{pick_new, replace_participant};
use crate::helpers::date::Date;
use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub channel: String,
}

#[derive(Debug)]
pub struct Response {
    pub id: String,
}

impl From<Participant> for Response {
    fn from(value: Participant) -> Self {
        Self { id: value.user }
    }
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Empty,
    NotFound,
    Unknown,
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

    if event.participants.len() == 0 {
        return Err(Error::Empty);
    }

    let mut participants = event.participants;
    let mut new_pick = pick_new(&participants);
    if let None = new_pick {
        participants = participants
            .into_iter()
            .map(|participant| Participant {
                picked: false,
                picked_at: None,
                ..participant
            })
            .collect();
        new_pick = pick_new(&participants);
    }
    let new_pick = match new_pick {
        Some(participant) => participant,
        None => return Err(Error::Empty),
    };
    event.participants = replace_participant(
        participants.clone(),
        Participant {
            picked: true,
            picked_at: Some(Date::now().timestamp()),
            ..new_pick.clone()
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
