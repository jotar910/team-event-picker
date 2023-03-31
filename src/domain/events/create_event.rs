use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_trim::{string_trim, vec_string_trim};

use crate::repository::errors::{FindError, InsertError};
use crate::repository::event::Repository;

use crate::domain::entities::{Event, RepeatPeriod};
use crate::domain::events::{insert_channel, insert_users};
use crate::domain::timezone::Timezone;

#[derive(Deserialize, Clone, Debug)]
pub struct Request {
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    pub timestamp: i64,
    pub timezone: String,
    pub repeat: String,
    #[serde(deserialize_with = "vec_string_trim")]
    pub participants: Vec<String>,
    #[serde(skip_deserializing)]
    pub channel: String,
    #[serde(skip_deserializing)]
    pub team_id: String,
}

impl From<Request> for insert_users::Request {
    fn from(value: Request) -> Self {
        Self {
            names: value.participants,
        }
    }
}

impl From<Request> for insert_channel::Request {
    fn from(value: Request) -> Self {
        Self {
            name: value.channel,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub created_channel: Option<String>,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadRequest,
    Conflict,
    Unknown,
}

impl From<insert_users::Error> for Error {
    fn from(value: insert_users::Error) -> Self {
        match value {
            insert_users::Error::Unknown => Error::Unknown,
        }
    }
}

impl From<insert_channel::Error> for Error {
    fn from(value: insert_channel::Error) -> Self {
        match value {
            insert_channel::Error::Unknown => Error::Unknown,
        }
    }
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let mut created_channel = None;
    let channel = match repo.clone().find_channel_by_name(req.channel.clone()).await {
        Ok(channel) => channel,
        Err(FindError::NotFound) => {
            created_channel = Some(req.channel.clone());
            insert_channel::execute(repo.clone(), req.clone().into())
                .await?
                .channel
        }
        Err(error) => {
            return Err(match error {
                FindError::NotFound => Error::BadRequest,
                FindError::Unknown => Error::Unknown,
            })
        }
    };
    match repo
        .clone()
        .find_event_by_name(req.name.clone(), channel.id)
        .await
    {
        Ok(..) => return Err(Error::Conflict),
        Err(error) if error != FindError::NotFound => return Err(Error::Unknown),
        _ => (),
    };

    let mut event = Event {
        id: 0,
        name: req.name.clone(),
        timestamp: req.timestamp,
        timezone: Timezone::from(req.timezone.clone()),
        repeat: RepeatPeriod::try_from(req.repeat.clone()).map_err(|_| Error::BadRequest)?,
        participants: vec![],
        channel: 0,
        prev_pick: 0,
        cur_pick: 0,
        team_id: req.team_id.clone(),
        deleted: false,
    };
    event.participants = insert_users::execute(repo.clone(), req.into())
        .await?
        .users
        .iter()
        .map(|user| user.id)
        .collect();
    event.channel = channel.id;

    match repo.insert_event(event).await {
        Ok(Event {
            id,
            timestamp,
            timezone,
            repeat,
            ..
        }) => Ok(Response {
            id,
            timestamp,
            timezone,
            repeat,
            created_channel,
        }),
        Err(err) => Err(match err {
            InsertError::Conflict => Error::Conflict,
            InsertError::Unknown => Error::Unknown,
        }),
    }
}
