use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::sync::Mutex;
use std::{collections::HashMap, sync::MutexGuard};

use itertools::Itertools;

use crate::domain::entities::{Event, EventCreation, User};

#[derive(Debug, PartialEq)]
pub enum FindError {
    NotFound,
    Unknown,
}
pub enum FindAllError {
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum InsertError {
    Conflict,
    Unknown,
}

#[allow(drop_bounds)]
pub trait Transition: Drop {
    fn commit(&self);
    fn rollback(&self);
}

pub trait Repository: Send + Sync {
    fn transition(&self) -> Box<dyn Transition>;

    fn find(&self, id: u32) -> Result<Event, FindError>;
    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError>;

    fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError>;
}

pub struct InMemoryRepository {
    events: Mutex<Vec<Event>>,
    users: Mutex<Vec<User>>,
}

impl InMemoryRepository {
    pub fn new() -> InMemoryRepository {
        InMemoryRepository {
            events: Mutex::new(vec![]),
            users: Mutex::new(vec![]),
        }
    }

    fn insert_users(&self, names: Vec<String>) -> Result<Vec<u32>, InsertError> {
        let mut users: HashMap<String, Option<User>> =
            names.iter().map(|name| (name.clone(), None)).collect();

        self.fill_with_existing_users(users.borrow_mut())?;

        let mut lock: MutexGuard<Vec<User>> = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        let start_id = lock.len() as u32;
        let mut add_users: Vec<User> = vec![];
        for name in names.iter().unique() {
            let user = users.get(name).unwrap();
            if let None = user {
                add_users.push(User {
                    id: start_id + add_users.len() as u32,
                    name: name.to_string(),
                })
            }
        }

        let added_from_idx = lock.len();
        for user in add_users.into_iter() {
            lock.push(user);
        }

        for existing_user in lock.iter().skip(added_from_idx) {
            users.insert(
                existing_user.name.clone(),
                Some(existing_user.to_owned()),
            );
        }

        Ok(names
            .into_iter()
            .map(|name| users[&name].as_ref().unwrap().id)
            .collect())
    }

    fn fill_with_existing_users(
        &self,
        users: &mut HashMap<String, Option<User>>,
    ) -> Result<(), InsertError> {
        let lock: MutexGuard<Vec<User>> = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        for existing_user in lock.iter() {
            if !users.contains_key(&existing_user.name) {
                continue;
            }
            users.insert(
                existing_user.name.clone(),
                Some(existing_user.clone()),
            );
        }

        Ok(())
    }

    fn update_event(
        &self,
        event: &mut Event,
        event_data: EventCreation,
    ) -> Result<Event, InsertError> {
        event.name = event_data.name;
        event.date = event_data.date;
        event.repeat = event_data.repeat;
        event.participants = self.insert_users(event_data.participants)?;
        event.deleted = false;
        Ok(event.clone())
    }
}

impl Repository for InMemoryRepository {
    fn transition(&self) -> Box<dyn Transition> {
        Box::new(InMemoryTransaction::new())
    }

    fn find(&self, id: u32) -> Result<Event, FindError> {
        let lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&event| event.id == id) {
            Some(event) => {
                if event.deleted {
                    return Err(FindError::NotFound);
                }
                Ok(event.clone())
            }
            _ => Err(FindError::NotFound),
        }
    }

    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        for existing_event in lock.iter_mut() {
            if existing_event.name == event_data.name {
                if existing_event.deleted {
                    return Ok(self.update_event(existing_event, event_data)?);
                }
                return Err(InsertError::Conflict);
            }
        }

        let id = lock.len() as u32;
        let event = Event {
            id,
            name: event_data.name,
            date: event_data.date,
            repeat: event_data.repeat,
            participants: self.insert_users(event_data.participants)?,
            deleted: false,
        };

        lock.push(event.clone());

        Ok(event)
    }

    fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError> {
        let lock = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(FindAllError::Unknown),
        };

        let ids_set: HashSet<&u32> = ids.iter().collect();

        let existing_users: Vec<User> = lock
            .iter()
            .filter(|user| ids_set.contains(&user.id))
            .map(|user| user.clone())
            .collect();

        let users = ids
            .into_iter()
            .filter_map(|key| {
                existing_users
                    .iter()
                    .find(|user| user.id == key)
            })
            .cloned()
            .collect();

        Ok(users)
    }
}

pub struct InMemoryTransaction {}

impl InMemoryTransaction {
    fn new() -> InMemoryTransaction {
        Self {}
    }
}

impl Transition for InMemoryTransaction {
    fn commit(&self) {
        // There's no way to do the commit here.
    }

    fn rollback(&self) {
        // There's no way to do the rollback here.
    }
}

impl Drop for InMemoryTransaction {
    fn drop(&mut self) {
        self.commit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;

    #[test]
    fn it_should_return_the_id_for_the_created_event() {
        let repo = InMemoryRepository::new();

        let result = repo.insert(mocks::mock_event_creation());

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_create_new_participants_when_creating_event() {
        let repo = InMemoryRepository::new();

        let result = repo.insert(mocks::mock_event_creation());

        match result {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 1]),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_use_existing_participants_when_creating_event() {
        let repo = InMemoryRepository::new();

        let mut creation = mocks::mock_event_creation();
        creation.participants[0] = "Joana".to_string();

        let result = repo.insert(creation);

        match result {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 0]),
            _ => unreachable!(),
        }

        // New event creation ---

        let mut creation = mocks::mock_event_creation();
        creation.name += "2";

        let result = repo.insert(creation);

        match result {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![1, 0]),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_conflict_when_created_events_with_the_same_name() {
        let repo = InMemoryRepository::new();

        let result = repo.insert(mocks::mock_event_creation());

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        let result = repo.insert(mocks::mock_event_creation());

        match result {
            Err(err) => assert_eq!(err, InsertError::Conflict),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_not_found_error_when_find_event_does_not_exist() {
        let repo = InMemoryRepository::new();

        let result = repo.find(0);

        match result {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_the_event_when_find_is_called_with_a_existing_id() {
        let repo = InMemoryRepository::new();

        let mock = mocks::mock_event_creation();
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += " 2";
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find here ---

        let result = repo.find(1);

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_find_participants_that_have_the_same_ids_as_the_provided() {
        let repo = InMemoryRepository::new();

        let mut mock = mocks::mock_event_creation();
        mock.participants.push("Francisca".to_string());
        mock.participants.push("Simão".to_string());
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find_participants here ---

        let result = repo.find_users(vec![1, 2]);

        match result {
            Ok(participants) => assert_eq!(
                participants,
                vec![
                    User {
                        id: 1,
                        name: "Joana".to_string()
                    },
                    User {
                        id: 2,
                        name: "Francisca".to_string()
                    }
                ]
            ),
            _ => unreachable!(),
        }
    }
}
