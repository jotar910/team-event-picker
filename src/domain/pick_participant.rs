use std::sync::Arc;

use crate::repository::errors::{FindError, UpdateError};
use crate::repository::event::Repository;

use super::entities::User;
use super::helpers;

pub struct Request {
    pub event: u32,
    pub channel: String,
}

#[derive(Debug)]
pub struct Response {
    pub id: u32,
    pub name: String,
}

impl From<User> for Response {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<Response> for User {
    fn from(value: Response) -> Self {
        Self {
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

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let channel = repo
        .find_channel_by_name(req.channel)
        .await
        .map_err(|error| {
            return match error {
                FindError::NotFound => Error::NotFound,
                FindError::Unknown => Error::Unknown,
            };
        })?;
    let event = repo
        .find_event(req.event, channel.id)
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

    let (pick, participant) = helpers::pick(&event);

    repo.save_pick(pick).await.map_err(|error| match error {
        UpdateError::NotFound => Error::NotFound,
        UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
    })?;

    Ok(repo
        .find_user(participant)
        .await
        .map_err(|error| match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        })?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_pick_randomly_participants() {
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
            Ok(event) => assert_ne!(event.cur_pick, 0),
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
            Ok(event) => assert_eq!(event.cur_pick, 3),
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
            Ok(event) => assert_eq!(event.cur_pick > 0 && event.cur_pick < 3, true),
            Err(..) => unreachable!("event must exist"),
        };
    }
}
