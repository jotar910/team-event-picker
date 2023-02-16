use std::sync::Arc;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::Channel;
use crate::repository::event::{FindAllError, Repository};

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>) -> Result<ListResponse<Channel>, Error> {
    match repo.find_all_channels().await {
        Err(err) => {
            return match err {
                FindAllError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(channels) => Ok(ListResponse::new(channels)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_return_all_the_channels() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing find here --

        let result = execute(repo).await;

        match result {
            Ok(ListResponse { data }) => assert_eq!(data, vec![mocks::mock_channel()]),
            _ => unreachable!(),
        }
    }
}
