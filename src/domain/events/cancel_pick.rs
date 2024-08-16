use std::sync::Arc;

use crate::domain::entities::Participant;
use crate::domain::helpers::participant::{last_picked, replace_participant};
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

    if let Some(participant) = last_picked(&event.participants) {
        event.participants = replace_participant(
            event.participants.clone(),
            Participant {
                picked: false,
                picked_at: None,
                ..participant.clone()
            },
        );
        repo.update_event(event).await.map_err(|error| {
            return match error {
                UpdateError::NotFound => Error::NotFound,
                UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
            };
        })?;
    }

    Ok(())
}
