use std::sync::Arc;

use crate::repository::event::Repository;
use crate::repository::event::UpdateError;

use super::entities::{Event, EventCreation, RepeatPeriod};

pub struct Request {
    pub id: u32,
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
    NotFound,
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let id = req.id;
    let creation_data = match EventCreation::try_from(req) {
        Ok(data) => data,
        Err(..) => return Err(Error::BadRequest),
    };
    let tx = repo.transition();
    match repo.update(id, creation_data) {
        Ok(Event { id, .. }) => Ok(Response { id }),
        Err(err) => {
            tx.rollback();
            Err(match err {
                UpdateError::Conflict => Error::Conflict,
                UpdateError::NotFound => Error::NotFound,
                UpdateError::Unknown => Error::Unknown,
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
    fn it_should_return_the_id_for_the_updated_event() {
        let repo = Arc::new(InMemoryRepository::new());

        if let Err(..) = repo.clone().insert(mocks::mock_event_creation()) {
            unreachable!("event must exist")
        }

        // Testing update here

        let req = mocks::mock_update_event_request();

        let result = execute(repo, req);

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_return_bad_request_error_on_invalid_request_payload_for_repeat_field() {
        let repo = Arc::new(InMemoryRepository::new());
        let mut req = mocks::mock_update_event_request();
        req.repeat = "test".to_string();

        let result = execute(repo, req);

        match result {
            Err(err) => assert_eq!(err, Error::BadRequest),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_return_not_found_error_when_the_event_to_update_does_not_exist() {
        let repo = Arc::new(InMemoryRepository::new());
        let req = mocks::mock_update_event_request();

        let result = execute(repo, req);

        match result {
            Err(err) => assert_eq!(err, Error::NotFound),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_return_conflict_error_when_the_event_to_update_does_not_exist() {
        let repo = Arc::new(InMemoryRepository::new());

        if let Err(..) = repo.clone().insert(mocks::mock_event_creation()) {
            unreachable!("event must exist")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "2";

        if let Err(..) = repo.clone().insert(mock) {
            unreachable!("event must exist")
        }

        // Testing update here

        let mut req = mocks::mock_update_event_request();
        req.id = 1;

        let result = execute(repo, req);

        match result {
            Err(err) => assert_eq!(err, Error::Conflict),
            _ => unreachable!(),
        };
    }
}
