use std::sync::Arc;

use crate::domain::insert_users;
use crate::repository::event::{FindError, Repository, UpdateError};

use super::helpers::pick_update::PickUpdateHelper;

pub struct Request {
    pub event: u32,
    pub participants: Vec<String>,
}

impl From<Request> for insert_users::Request {
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
    let event = repo.clone().find_event(event_id);

    if let Err(error) = event {
        return Err(match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        });
    }

    let mut event = event.unwrap();

    let pick_update_helper = PickUpdateHelper::new(&event.participants, event.cur_pick);

    event.participants = insert_users::execute(repo.clone(), req.into())
        .map_err(|err| match err {
            insert_users::Error::Unknown => Error::Unknown,
        })?
        .users
        .iter()
        .map(|user| user.id)
        .collect();

    event.cur_pick = pick_update_helper.new_pick(&event.participants);
    event.prev_pick = event.cur_pick;

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
    use crate::domain::entities::{Event, EventPick};
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_update_participants() {
        let repo = Arc::new(InMemoryRepository::new());

        let result = mocks::insert_mock_event(repo.clone());

        assert_eq!(result.participants, vec![0, 1]);

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

        match repo.find_event(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![2, 3, 1]),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_update_pick_data_when_participants_are_updated() {
        let repo = Arc::new(InMemoryRepository::new());

        let result = mocks::insert_mock_event(repo.clone());

        assert_eq!(result.participants, vec![0, 1]);

        if let Err(..) = repo.clone().save_pick(EventPick{event: 0, pick: 3}) {
            unreachable!("event pick data must be saved")
        }

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

        match repo.find_event(0) {
            Ok(Event { cur_pick, prev_pick, .. }) => {
                assert_eq!(prev_pick, 4);
                assert_eq!(cur_pick, 4);
            },
            _ => unreachable!(),
        }

        let req = Request {
            event: 0,
            participants: mocks::mock_participants().into_iter().map(|p| p.name).rev().collect(),
        };

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find_event(0) {
            Ok(Event { cur_pick, prev_pick, .. }) => {
                assert_eq!(prev_pick, 1);
                assert_eq!(cur_pick, 1);
            },
            _ => unreachable!(),
        }
    }
}
