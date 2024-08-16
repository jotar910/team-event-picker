use std::sync::Arc;

use serde::Serialize;

use crate::domain::entities::{Participant, RepeatPeriod};
use crate::domain::timezone::Timezone;
use crate::repository::errors::FindError;
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
    pub name: String,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub participants: Vec<Participant>,
    pub channel: String,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event = match repo.find_event(req.id, req.channel.clone()).await {
        Err(err) => {
            return match err {
                FindError::NotFound => Err(Error::NotFound),
                FindError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(event) => event,
    };

    Ok(Response {
        id: event.id,
        name: event.name,
        timestamp: event.timestamp,
        timezone: event.timezone,
        repeat: event.repeat,
        participants: event.participants,
        channel: req.channel,
    })
}
