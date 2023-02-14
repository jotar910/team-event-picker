use std::sync::Arc;

use crate::repository::event::{DeleteError, Repository};

#[derive(Debug, PartialEq)]
pub enum Error {
    NotFound,
    Unknown,
}
pub struct Request {
    pub id: u32,
}

#[derive(Debug, PartialEq)]
pub struct Response {
    pub id: u32,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event = match repo.delete_event(req.id) {
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
    use crate::repository::event::{FindError, InMemoryRepository};

    #[test]
    fn it_should_delete_the_event_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone());

        // Testing delete here --

        let req = Request { id: 0 };

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find(0) {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!("event must not exist"),
        }
    }

    #[test]
    fn it_should_return_not_found_error_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());
        let req = Request { id: 0 };

        let result = execute(repo, req);

        match result {
            Err(error) => assert_eq!(error, Error::NotFound),
            _ => unreachable!(),
        }
    }
}
