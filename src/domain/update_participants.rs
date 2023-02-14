use std::sync::Arc;

use crate::domain::insert_participants;
use crate::repository::event::{Repository, UpdateError, FindError};

pub struct Request {
    pub event: u32,
    pub participants: Vec<String>,
}

impl From<Request> for insert_participants::Request {
    fn from(value: Request) -> Self {
        Self {
            names: value.participants,
        }
    }
}

pub struct Response {
    pub id: u32,
}

pub enum Error {
    NotFound,
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event_id = req.event;
    let event = repo.clone().find(event_id);

    if let Err(error) = event {
        return Err(match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        });
    }

    let mut event = event.unwrap();

    event.participants = insert_participants::execute(repo.clone(), req.into())
        .map_err(|err| match err {
            insert_participants::Error::Unknown => Error::Unknown,
        })?
        .users
        .iter()
        .map(|user| user.id)
        .collect();

    match repo.update_event(event) {
        Err(error) => match error {
            UpdateError::NotFound => Err(Error::NotFound),
            UpdateError::Conflict | UpdateError::Unknown => Err(Error::Unknown),
        },
        Ok(..) => Ok(Response { id: event_id }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_update_participants() {
        let repo = Arc::new(InMemoryRepository::new());

        match repo.insert(mocks::mock_event_creation()) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 1]),
            _ => unreachable!("event must be created for this test"),
        }

        // Testing update_participants here ---

        let req = mocks::mock_participants_update();

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![2, 3, 1]),
            _ => unreachable!(),
        }
    }
}
