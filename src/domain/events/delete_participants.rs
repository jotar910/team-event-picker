use std::sync::Arc;

use serde::Serialize;

use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub channel: String,
    pub participants: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
    pub channel: String,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event_id = req.event;

    let event = repo.find_event(event_id, req.channel.clone()).await;

    if let Err(error) = event {
        return Err(match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        });
    }

    let mut event = event.unwrap();

    event.participants = event
        .participants
        .into_iter()
        .filter(|participant| !req.participants.contains(&participant.user))
        .collect();

    match repo.update_event(event).await {
        Err(error) => match error {
            UpdateError::NotFound => Err(Error::NotFound),
            UpdateError::Conflict | UpdateError::Unknown => Err(Error::Unknown),
        },
        Ok(..) => Ok(Response {
            id: event_id,
            channel: req.channel,
        }),
    }
}
