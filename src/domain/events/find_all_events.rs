use std::sync::Arc;

use serde::Serialize;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::{Participant, RepeatPeriod};
use crate::domain::timezone::Timezone;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

pub struct Request {
    pub channels: Vec<String>,
}

impl Request {
    pub fn new() -> Self {
        Self { channels: vec![] }
    }

    pub fn with_channel(self, channel: String) -> Self {
        Self { channels: vec![channel], ..self }
    }

    pub fn with_channels(self, channels: Vec<String>) -> Self {
        Self { channels, ..self }
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Response {
    pub id: u32,
    pub name: String,
    pub channel: String,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub participants: Vec<Participant>,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub async fn execute(
    repo: Arc<dyn Repository>,
    req: Request,
) -> Result<ListResponse<Response>, Error> {
    let events = match repo.find_all_events(req.channels).await {
        Err(err) => {
            return match err {
                FindAllError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(events) => events,
    };
    Ok(ListResponse::new(
        events
            .into_iter()
            .map(|event| Response {
                id: event.id,
                name: event.name,
                channel: event.channel,
                timestamp: event.timestamp,
                timezone: event.timezone,
                repeat: event.repeat,
                participants: event.participants,
            })
            .collect(),
    ))
}
