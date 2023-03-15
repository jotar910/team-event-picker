use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_trim::{string_trim, vec_string_trim};

use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

use crate::domain::entities::{Event, RepeatPeriod};
use crate::domain::{insert_channel, insert_users, timezone::Timezone};

#[derive(Deserialize, Clone)]
pub struct Request {
    pub id: u32,
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    pub timestamp: i64,
    pub timezone: String,
    pub repeat: String,
    #[serde(deserialize_with = "vec_string_trim")]
    pub participants: Vec<String>,
    #[serde(skip_deserializing)]
    pub channel: String,
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
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadRequest,
    Conflict,
    NotFound,
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
    let channel = repo
        .find_channel_by_name(req.channel.clone())
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    let existing_event = match repo.clone().find_event(req.id.clone(), channel.id).await {
        Ok(event) => event,
        Err(error) => {
            return Err(match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            })
        }
    };

    let mut event = Event {
        id: existing_event.id,
        name: req.name.clone(),
        timestamp: req.timestamp,
        timezone: Timezone::from(req.timezone.clone()),
        repeat: RepeatPeriod::try_from(req.repeat.clone()).map_err(|_| Error::BadRequest)?,
        participants: vec![],
        channel: existing_event.channel,
        prev_pick: 0,
        cur_pick: 0,
        deleted: false,
    };
    event.participants = insert_users::execute(repo.clone(), req.clone().into())
        .await?
        .users
        .iter()
        .map(|user| user.id)
        .collect();
    event.channel = insert_channel::execute(repo.clone(), req.clone().into())
        .await?
        .channel
        .id;

    match repo.update_event(event.clone()).await {
        Ok(..) => Ok(Response {
            id: event.id,
            timestamp: event.timestamp,
            timezone: event.timezone,
            repeat: event.repeat,
        }),
        Err(err) => Err(match err {
            UpdateError::Conflict => Error::Conflict,
            UpdateError::NotFound => Error::NotFound,
            UpdateError::Unknown => Error::Unknown,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_return_the_id_for_the_updated_event() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing update here

        let req = mocks::mock_update_event_request();

        let result = execute(repo, req).await;

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };
    }

    #[tokio::test]
    async fn it_should_return_bad_request_error_on_invalid_request_payload_for_repeat_field() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing update here

        let mut req = mocks::mock_update_event_request();
        req.repeat = "test".to_string();

        let result = execute(repo, req).await;

        match result {
            Err(err) => assert_eq!(err, Error::BadRequest),
            _ => unreachable!(),
        };
    }

    #[tokio::test]
    async fn it_should_return_not_found_error_when_the_event_to_update_does_not_exist() {
        let repo = Arc::new(InMemoryRepository::new());
        let req = mocks::mock_update_event_request();

        let result = execute(repo, req).await;

        match result {
            Err(err) => assert_eq!(err, Error::NotFound),
            _ => unreachable!(),
        };
    }

    #[tokio::test]
    async fn it_should_return_conflict_error_when_the_event_to_update_does_not_exist() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        let mut mock = mocks::mock_event();
        mock.name += "2";

        if let Err(..) = repo.clone().insert_event(mock).await {
            unreachable!("event must exist")
        }

        // Testing update here

        let mut req = mocks::mock_update_event_request();
        req.id = 1;

        let result = execute(repo, req).await;

        match result {
            Err(err) => assert_eq!(err, Error::Conflict),
            _ => unreachable!(),
        };
    }

    #[tokio::test]
    async fn it_should_update_event_with_the_provided_data() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing update here --

        let mut req = mocks::mock_update_event_request();
        req.name = "Johny".to_string();

        let result = execute(repo.clone(), req).await;

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };

        match repo.find_event(0, 0).await {
            Ok(Event { name, .. }) => assert_eq!(name, "Johny"),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn it_should_return_not_found_error_when_event_to_update_does_not_exist() {
        let repo = Arc::new(InMemoryRepository::new());

        let req = mocks::mock_update_event_request();

        let result = execute(repo, req).await;

        match result {
            Err(error) => assert_eq!(error, Error::NotFound),
            _ => unreachable!(),
        }
    }
}
