use std::sync::Arc;

use crate::domain::entities::User;
use crate::domain::pick_participant;
use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

pub struct Request {
    pub event: u32,
    pub channel: String,
}

impl From<Request> for pick_participant::Request {
    fn from(value: Request) -> Self {
        Self {
            event: value.event,
            channel: value.channel,
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub id: u32,
    pub name: String,
}

impl From<pick_participant::Response> for Response {
    fn from(value: pick_participant::Response) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<Response> for User {
    fn from(value: Response) -> User {
        User {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Empty,
    NotFound,
    Unknown,
}

impl From<pick_participant::Error> for Error {
    fn from(value: pick_participant::Error) -> Self {
        match value {
            pick_participant::Error::Empty => Self::Empty,
            pick_participant::Error::NotFound => Self::NotFound,
            pick_participant::Error::Unknown => Self::Unknown,
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
    let event = repo
        .find_event(req.event.clone(), channel.id)
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;

    if event.participants.len() == 0 {
        return Err(Error::Empty);
    }

    repo.clone()
        .rev_pick(event.id, channel.id)
        .await
        .map_err(|error| match error {
            UpdateError::NotFound => Error::NotFound,
            UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
        })?;

    Ok(pick_participant::execute(repo, req.into()).await?.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_re_pick_randomly_participants() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing pick here ---

        let result = execute(
            repo.clone(),
            Request {
                event: 0,
                channel: String::from("Channel"),
            },
        )
        .await;

        if let Err(..) = result {
            unreachable!()
        }

        match repo.find_event(0, 0).await {
            Ok(event) => assert!(event.cur_pick > 0 && event.cur_pick < 3),
            Err(..) => unreachable!("event must exist"),
        };

        if let Err(..) = execute(
            repo.clone(),
            Request {
                event: 0,
                channel: String::from("Channel"),
            },
        )
        .await
        {
            unreachable!()
        }

        match repo.find_event(0, 0).await {
            Ok(event) => assert!(event.cur_pick > 0 && event.cur_pick < 3),
            Err(..) => unreachable!("event must exist"),
        };
    }
}
