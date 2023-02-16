use std::sync::Arc;

use rand::Rng;

use crate::repository::event::{FindError, Repository, UpdateError};

use super::entities::{EventPick, User};

pub struct Request {
    pub event: u32,
}

#[derive(Debug)]
pub struct Response {
    pub id: u32,
    pub name: String,
}

impl From<User> for Response {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Empty,
    NotFound,
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event = repo.find_event(req.event).await.map_err(|error| {
        return match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        };
    })?;

    if event.participants.len() == 0 {
        return Err(Error::Empty);
    }

    let pick = event.cur_pick;
    let total_participants = event.participants.len();

    let mut not_picked: Vec<usize> = vec![];

    for i in 0..event.participants.len() {
        if pick & (1 << i) == 0 {
            not_picked.push(i);
        }
    }

    let mut rng = rand::thread_rng();
    let new_pick_idx: usize;
    let new_pick: u32;

    if not_picked.len() == 0 {
        new_pick_idx = rng.gen_range(0..total_participants);
        new_pick = 1 << new_pick_idx;
    } else {
        new_pick_idx = not_picked[rng.gen_range(0..not_picked.len())];
        new_pick = pick | (1 << new_pick_idx);
    }

    repo.save_pick(EventPick {
        event: event.id,
        pick: new_pick,
    })
    .await
    .map_err(|error| match error {
        UpdateError::NotFound => Error::NotFound,
        UpdateError::Conflict | UpdateError::Unknown => Error::Unknown,
    })?;

    Ok(repo
        .find_user(event.participants[new_pick_idx])
        .await
        .map_err(|error| match error {
            FindError::NotFound => Error::NotFound,
            FindError::Unknown => Error::Unknown,
        })?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_pick_randomly_participants() {
        let repo = Arc::new(InMemoryRepository::new());

        mocks::insert_mock_event(repo.clone()).await;

        // Testing pick here ---

        let result = execute(repo.clone(), Request { event: 0 }).await;

        if let Err(..) = result {
            unreachable!()
        }

        match repo.find_event(0).await {
            Ok(event) => assert_ne!(event.cur_pick, 0),
            Err(..) => unreachable!("event must exist"),
        };

        if let Err(..) = execute(repo.clone(), Request { event: 0 }).await {
            unreachable!()
        }

        match repo.find_event(0).await {
            Ok(event) => assert_eq!(event.cur_pick, 3),
            Err(..) => unreachable!("event must exist"),
        };

        if let Err(..) = execute(repo.clone(), Request { event: 0 }).await {
            unreachable!()
        }

        match repo.find_event(0).await {
            Ok(event) => assert_eq!(event.cur_pick > 0 && event.cur_pick < 3, true),
            Err(..) => unreachable!("event must exist"),
        };
    }
}
