use std::sync::Arc;

use serde::Serialize;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::RepeatPeriod;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

use super::timezone::Timezone;

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
}

#[derive(Debug)]
pub enum Error {
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>) -> Result<ListResponse<Response>, Error> {
    let events = match repo.find_all_events_unprotected().await {
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
                timestamp: event.timestamp,
                timezone: event.timezone,
                repeat: event.repeat,
            })
            .collect(),
    ))
}
