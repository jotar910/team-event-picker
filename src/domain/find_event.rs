use std::sync::Arc;

use crate::domain::entities::{Participant, RepeatPeriod};
use crate::repository::event::{FindAllError, FindError, Repository};

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
    pub name: String,
    pub date: String,
    pub repeat: RepeatPeriod,
    pub participants: Vec<Participant>,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event = match repo.find(req.id) {
        Err(err) => {
            return match err {
                FindError::NotFound => Err(Error::NotFound),
                FindError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(event) => event,
    };
    Ok(Response {
        id: event.id,
        name: event.name,
        date: event.date,
        repeat: event.repeat,
        participants: repo
            .find_participants(event.participants)
            .map_err(|error| match error {
                FindAllError::Unknown => Error::Unknown,
            })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_return_the_event_for_the_provided_id() {
        let repo = Arc::new(InMemoryRepository::new());

        if let Err(..) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created for this test")
        }

        // Testing find here --

        let req = Request { id: 0 };

        let result = execute(repo, req);

        match result {
            Ok(res) => assert_eq!(res, mocks::mock_find_event_response()),
            _ => unreachable!(),
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
