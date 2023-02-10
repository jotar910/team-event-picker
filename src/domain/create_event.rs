use std::sync::Arc;

use crate::repository::event::InsertError;
use crate::repository::event::Repository;

use super::entities::{Event, EventCreation, RepeatPeriod};

pub struct Request {
    pub name: String,
    pub date: String,
    pub repeat: String,
    pub participants: Vec<String>,
}

impl TryFrom<Request> for EventCreation {
    type Error = ();

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        let repeat = RepeatPeriod::try_from(value.repeat)?;

        Ok(EventCreation {
            repeat,
            name: value.name,
            date: value.date,
            participants: value.participants,
        })
    }
}

pub struct Response {
    pub id: u32,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadRequest,
    Conflict,
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let creation_data = match EventCreation::try_from(req) {
        Ok(data) => data,
        Err(..) => return Err(Error::BadRequest),
    };
    let tx = repo.transition();
    match repo.insert(creation_data) {
        Ok(Event { id, .. }) => Ok(Response { id }),
        Err(err) => {
            tx.rollback();
            Err(match err {
                InsertError::Conflict => Error::Conflict,
                _ => Error::Unknown,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_return_the_id_for_the_created_event() {
        let repo = Arc::new(InMemoryRepository::new());
        let req = mocks::mock_request();

        let result = execute(repo, req);

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_fail_on_invalid_request_payload_for_repeat_field() {
        let repo = Arc::new(InMemoryRepository::new());
        let mut req = mocks::mock_request();
        req.repeat = "test".to_string();

        let result = execute(repo, req);

        match result {
            Err(err) => assert_eq!(err, Error::BadRequest),
            _ => unreachable!(),
        };
    }
}
