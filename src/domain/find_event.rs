use std::sync::Arc;

use serde::Serialize;

use crate::domain::entities::{Channel, RepeatPeriod, User};
use crate::repository::errors::{FindAllError, FindError};
use crate::repository::event::Repository;

use super::timezone::Timezone;

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
    pub name: String,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub participants: Vec<User>,
    pub channel: Channel,
    pub picked: Vec<User>,
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

    let event = match repo.find_event(req.id, channel.id).await {
        Err(err) => {
            return match err {
                FindError::NotFound => Err(Error::NotFound),
                FindError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(event) => event,
    };

    let participants = repo
        .find_users(event.participants)
        .await
        .map_err(|error| match error {
            FindAllError::Unknown => Error::Unknown,
        })?;

    Ok(Response {
        id: event.id,
        name: event.name,
        timestamp: event.timestamp,
        timezone: event.timezone,
        repeat: event.repeat,
        participants: participants.clone(),
        channel: repo
            .find_channel(event.channel)
            .await
            .map_err(|error| match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            })?,
        picked: participants
            .into_iter()
            .enumerate()
            .filter(|(i, _)| event.cur_pick & (1 << i) > 0)
            .map(|(_, participant)| participant)
            .collect(),
    })
}
