use std::sync::Arc;

use serde::Serialize;

use crate::domain::events::insert_users;
use crate::domain::helpers::pick_update::PickUpdateHelper;
use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub participants: Vec<String>,
    pub channel: String,
}

impl From<Request> for insert_users::Request {
    fn from(value: Request) -> Self {
        Self {
            names: value.participants,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event_id = req.event;

    let channel = repo
        .find_channel_by_name(req.channel.clone())
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    let event = repo.clone().find_event(event_id, channel.id).await;

    if let Err(error) = event {
        return Err(match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        });
    }

    let mut event = event.unwrap();

    let pick_update_helper = PickUpdateHelper::new(&event.participants, event.cur_pick);

    event.participants = insert_users::execute(repo.clone(), req.into())
        .await
        .map_err(|err| match err {
            insert_users::Error::Unknown => Error::Unknown,
        })?
        .users
        .iter()
        .map(|user| user.id)
        .collect();

    event.cur_pick = pick_update_helper.new_pick(&event.participants);
    event.prev_pick = event.cur_pick;

    match repo.update_event(event).await {
        Err(error) => match error {
            UpdateError::NotFound => Err(Error::NotFound),
            UpdateError::Conflict | UpdateError::Unknown => Err(Error::Unknown),
        },
        Ok(..) => Ok(Response { id: event_id }),
    }
}
