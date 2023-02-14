use std::sync::Arc;

use crate::repository::event::{FindAllError, FindError, Repository, UpdateError};

pub struct Request {
    pub event: u32,
    pub participants: Vec<String>,
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

    let event = repo.find(event_id);

    if let Err(error) = event {
        return Err(match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        });
    }

    let mut event = event.unwrap();

    event.participants = repo
        .find_users(event.participants.clone())
        .map_err(|err| match err {
            FindAllError::Unknown => Error::Unknown,
        })?
        .into_iter()
        .filter(|participant| !req.participants.contains(&participant.name))
        .map(|participant| participant.id)
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

        mocks::insert_mock_event(repo.clone());

        // Testing update_participants here ---

        let req = Request {
            event: 0,
            participants: mocks::mock_users_names(),
        };

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0]),
            _ => unreachable!(),
        }
    }
}
