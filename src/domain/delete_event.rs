use std::sync::Arc;

use serde::Serialize;

use crate::repository::errors::{DeleteError, FindError};
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

    let event = match repo.delete_event(req.id, channel.id).await {
        Err(err) => {
            return match err {
                DeleteError::NotFound => Err(Error::NotFound),
                DeleteError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(event) => event,
    };
    Ok(Response { id: event.id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_delete_the_event_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing delete here --

        let req = Request {
            id: 0,
            channel: String::from("Channel"),
        };

        let result = execute(repo.clone(), req).await;

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find_event(0, 0).await {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!("event must not exist"),
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
