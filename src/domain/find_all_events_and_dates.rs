use std::sync::Arc;

use serde::Serialize;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::RepeatPeriod;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
    pub date: String,
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
                date: event.date,
                repeat: event.repeat,
            })
            .collect(),
    ))
}
