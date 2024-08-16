use std::sync::Arc;

use serde::Serialize;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::{Participant, RepeatPeriod};
use crate::domain::timezone::Timezone;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

pub struct Request {
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
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub async fn execute(
    repo: Arc<dyn Repository>,
    req: Request,
) -> Result<ListResponse<Response>, Error> {
    let events = match repo.find_all_events(req.channel).await {
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
                timestamp: event.timestamp,
                timezone: event.timezone,
                repeat: event.repeat,
                participants: event.participants,
            })
            .collect(),
    ))
}
