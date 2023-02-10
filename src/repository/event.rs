use std::borrow::BorrowMut;
use std::sync::Mutex;
use std::{collections::HashMap, sync::MutexGuard};

use crate::domain::entities::{Event, EventCreation, Participant};

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
    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError>;
}

pub struct InMemoryRepository {
    events: Mutex<Vec<Event>>,
    participants: Mutex<Vec<Participant>>,
}

impl InMemoryRepository {
    pub fn new() -> InMemoryRepository {
        InMemoryRepository {
            events: Mutex::new(vec![]),
            participants: Mutex::new(vec![]),
        }
    }

    fn insert_participants(&self, names: Vec<String>) -> Result<Vec<u32>, InsertError> {
        let mut participants: HashMap<String, Option<Participant>> =
            names.iter().map(|name| (name.clone(), None)).collect();

        self.fill_with_existing_participants(participants.borrow_mut())?;

        let mut lock: MutexGuard<Vec<Participant>> = match self.participants.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        let start_id = lock.len() as u32;
        let mut add_participants: Vec<Participant> = vec![];
        for (name, participant) in participants.iter() {
            if let None = participant {
                add_participants.push(Participant {
                    id: start_id + add_participants.len() as u32,
                    name: name.to_string(),
                })
            }
        }

        let added_from_idx = lock.len();
        for participant in add_participants.into_iter() {
            lock.push(participant);
        }

        for existing_participant in lock.iter().skip(added_from_idx) {
            participants.insert(
                existing_participant.name.clone(),
                Some(existing_participant.to_owned()),
            );
        }

        Ok(names
            .into_iter()
            .map(|name| participants[&name].as_ref().unwrap().id)
            .collect())
    }

    fn fill_with_existing_participants(
        &self,
        participants: &mut HashMap<String, Option<Participant>>,
    ) -> Result<(), InsertError> {
        let lock: MutexGuard<Vec<Participant>> = match self.participants.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        for existing_participant in lock.iter() {
            if !participants.contains_key(&existing_participant.name) {
                continue;
            }
            participants.insert(
                existing_participant.name.clone(),
                Some(existing_participant.clone()),
            );
        }

        Ok(())
    }
}

impl Repository for InMemoryRepository {
    fn transition(&self) -> Box<dyn Transition> {
        Box::new(InMemoryTransaction::new())
    }

    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        let id = lock.len() as u32;
        let event = Event {
            id,
            name: event_data.name,
            date: event_data.date,
            repeat: event_data.repeat,
            participants: self.insert_participants(event_data.participants)?,
        };

        lock.push(event.clone());

        Ok(event)
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
            Ok(Event { participants, .. }) => {
                assert_eq!(participants.contains(&0), true);
                assert_eq!(participants.contains(&1), true);
            },
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

        let creation = mocks::mock_event_creation();

        let result = repo.insert(creation);

        match result {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![1, 0]),
            _ => unreachable!(),
        }
    }
}
