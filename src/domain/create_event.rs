use std::sync::Arc;

use crate::repository::event::{FindError, InsertError, Repository};

use crate::domain::entities::{Event, RepeatPeriod};
use crate::domain::{insert_channel, insert_users};

#[derive(Clone)]
pub struct Request {
    pub name: String,
    pub date: String,
    pub repeat: String,
    pub participants: Vec<String>,
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

pub struct Response {
    pub id: u32,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadRequest,
    Conflict,
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

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    match repo.clone().find_event_by_name(req.name.clone()) {
        Ok(..) => return Err(Error::Conflict),
        Err(error) if error != FindError::NotFound => return Err(Error::Unknown),
        _ => (),
    };

    let mut event = Event {
        id: 0,
        name: req.name.clone(),
        date: req.date.clone(),
        repeat: RepeatPeriod::try_from(req.repeat.clone()).map_err(|_| Error::BadRequest)?,
        participants: vec![],
        channel: 0,
        prev_pick: 0,
        cur_pick: 0,
        deleted: false,
    };
    event.participants = insert_users::execute(repo.clone(), req.clone().into())?
        .users
        .iter()
        .map(|user| user.id)
        .collect();
    event.channel = insert_channel::execute(repo.clone(), req.into())?
        .channel
        .id;

    match repo.insert_event(event) {
        Ok(Event { id, .. }) => Ok(Response { id }),
        Err(err) => Err(match err {
            InsertError::Conflict => Error::Conflict,
            InsertError::Unknown => Error::Unknown,
        }),
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
        let req = mocks::mock_create_event_request();

        let result = execute(repo, req);

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_fail_on_invalid_request_payload_for_repeat_field() {
        let repo = Arc::new(InMemoryRepository::new());
        let mut req = mocks::mock_create_event_request();
        req.repeat = "test".to_string();

        let result = execute(repo, req);

        match result {
            Err(err) => assert_eq!(err, Error::BadRequest),
            _ => unreachable!(),
        };
    }

    #[test]
    fn it_should_create_new_participants_when_creating_event() {
        let repo = Arc::new(InMemoryRepository::new());

        let result = execute(repo.clone(), mocks::mock_create_event_request());

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };

        match repo.find_event(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 1]),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_use_existing_participants_when_creating_event() {
        let repo = Arc::new(InMemoryRepository::new());

        let mut req = mocks::mock_create_event_request();
        req.participants[0] = "Joana".to_string();

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        };

        match repo.clone().find_event(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 0]),
            _ => unreachable!(),
        }

        // Testing new event creation here ---

        let mut req = mocks::mock_create_event_request();
        req.name += "2";

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id }) => assert_eq!(id, 1),
            _ => unreachable!(),
        };

        match repo.clone().find_event(1) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![1, 0]),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_conflict_when_created_events_with_the_same_name() {
        let repo = Arc::new(InMemoryRepository::new());

        let result = execute(repo.clone(), mocks::mock_create_event_request());

        match result {
            Ok(Response { id }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        let result = execute(repo.clone(), mocks::mock_create_event_request());

        match result {
            Err(err) => assert_eq!(err, Error::Conflict),
            _ => unreachable!(),
        }
    }
}
