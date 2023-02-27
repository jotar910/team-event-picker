use std::sync::Arc;

use serde::Serialize;

use crate::domain::entities::{Channel, RepeatPeriod, User};
use crate::repository::errors::{FindAllError, FindError};
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
    pub date: String,
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
        date: event.date,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_return_the_event_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing find here --

        let req = Request {
            id: 0,
            channel: String::from("Channel"),
        };

        let result = execute(repo, req).await;

        match result {
            Ok(res) => assert_eq!(res, mocks::mock_find_event_response()),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn it_should_return_not_found_error_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());
        let req = Request {
            id: 0,
            channel: String::from("Channel"),
        };

        let result = execute(repo, req).await;

        match result {
            Err(error) => assert_eq!(error, Error::NotFound),
            _ => unreachable!(),
        }
    }
}
