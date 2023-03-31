use std::sync::Arc;

use serde::Serialize;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::RepeatPeriod;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

use super::timezone::Timezone;

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
    pub participants: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub async fn execute(
    repo: Arc<dyn Repository>,
    req: Request,
) -> Result<ListResponse<Response>, Error> {
    let channel = match repo.clone().find_channel_by_name(req.channel).await {
        Ok(channel) => channel,
        _ => return Ok(ListResponse::new(vec![])),
    };
    let events = match repo.find_all_events(channel.id).await {
        Err(err) => {
            return match err {
                FindAllError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(events) => events,
    };
    Ok(ListResponse::new({
        let mut responses = Vec::new();
        for event in events.into_iter() {
            let participants = match repo.find_users(event.participants).await {
                Ok(users) => users,
                Err(error) => match error {
                    FindAllError::Unknown => return Err(Error::Unknown),
                },
            };
            let response = Response {
                id: event.id,
                name: event.name,
                timestamp: event.timestamp,
                timezone: event.timezone,
                repeat: event.repeat,
                participants: participants.into_iter().map(|user| user.name).collect(),
            };
            responses.push(response);
        }
        responses
    }))
}
