use std::sync::Mutex;

use crate::domain::entities::{Event, EventCreation};

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
    fn insert(&self, event_data: EventCreation) -> Result<u32, InsertError>;
}

pub struct InMemoryRepository {
    events: Mutex<Vec<Event>>,
}

impl InMemoryRepository {
    pub fn new() -> InMemoryRepository {
        InMemoryRepository {
            events: Mutex::new(vec![]),
        }
    }
}

impl Repository for InMemoryRepository {
    fn transition(&self) -> Box<dyn Transition> {
        Box::new(InMemoryTransaction::new())
    }

    fn insert(&self, event_data: EventCreation) -> Result<u32, InsertError> {
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
            participants: vec![],
        };

        lock.push(event);

        Ok(id)
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
            Ok(id) => assert_eq!(id, 0),
            _ => unreachable!(),
        }
    }
}
