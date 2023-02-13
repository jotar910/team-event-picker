use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::sync::Mutex;
use std::{collections::HashMap, sync::MutexGuard};

use itertools::Itertools;

use crate::domain::entities::{Channel, Event, EventCreation, User};

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

#[derive(Debug, PartialEq)]
pub enum UpdateError {
    Conflict,
    NotFound,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum DeleteError {
    NotFound,
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
    fn find_all(&self, channel: String) -> Result<Vec<Event>, FindAllError>;
    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError>;
    fn update(&self, id: u32, event_data: EventCreation) -> Result<Event, UpdateError>;
    fn delete(&self, id: u32) -> Result<Event, DeleteError>;

    fn find_channel(&self, ids: u32) -> Result<Channel, FindError>;
    fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError>;

    fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError>;
}

pub struct InMemoryRepository {
    events: Mutex<Vec<Event>>,
    channels: Mutex<Vec<Channel>>,
    users: Mutex<Vec<User>>,
}

impl InMemoryRepository {
    pub fn new() -> InMemoryRepository {
        InMemoryRepository {
            events: Mutex::new(vec![]),
            channels: Mutex::new(vec![]),
            users: Mutex::new(vec![]),
        }
    }

    fn find_channel_by_name(&self, name: String) -> Option<Channel> {
        let lock: MutexGuard<Vec<Channel>> = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return None,
        };

        lock.iter()
            .find(|&channel| channel.name == name)
            .map(|channel| channel.clone())
    }

    fn insert_channel(&self, name: String) -> Result<u32, InsertError> {
        let mut lock: MutexGuard<Vec<Channel>> = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        if let Some(channel) = lock.iter().find(|&channel| channel.name == name) {
            return Ok(channel.id);
        }

        let id = lock.len() as u32;
        let channel = Channel { id, name };

        lock.push(channel);

        Ok(id)
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
            users.insert(existing_user.name.clone(), Some(existing_user.to_owned()));
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
            users.insert(existing_user.name.clone(), Some(existing_user.clone()));
        }

        Ok(())
    }

    fn update_event(
        &self,
        event: &mut Event,
        event_data: EventCreation,
    ) -> Result<Event, UpdateError> {
        event.name = event_data.name;
        event.date = event_data.date;
        event.repeat = event_data.repeat;
        event.participants =
            self.insert_users(event_data.participants)
                .map_err(|error| match error {
                    InsertError::Conflict => UpdateError::Conflict,
                    InsertError::Unknown => UpdateError::Unknown,
                })?;
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

    fn find_all(&self, channel: String) -> Result<Vec<Event>, FindAllError> {
        let lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(FindAllError::Unknown),
        };
        let channel = match self.find_channel_by_name(channel) {
            Some(channel) => channel,
            None => return Ok(vec![]),
        };
        Ok(lock
            .iter()
            .filter(|event| event.channel == channel.id)
            .map(|event| event.clone())
            .collect())
    }

    fn insert(&self, event_data: EventCreation) -> Result<Event, InsertError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        for existing_event in lock.iter_mut() {
            if existing_event.name == event_data.name {
                if existing_event.deleted {
                    return Ok(self
                        .update_event(existing_event, event_data)
                        .map_err(|error| match error {
                            UpdateError::Conflict => InsertError::Conflict,
                            UpdateError::NotFound | UpdateError::Unknown => InsertError::Unknown,
                        })?);
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
            channel: self.insert_channel(event_data.channel)?,
            deleted: false,
        };

        lock.push(event.clone());

        Ok(event)
    }

    fn update(&self, id: u32, event_data: EventCreation) -> Result<Event, UpdateError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(UpdateError::Unknown),
        };

        let mut event_to_update: Option<&mut Event> = None;

        for existing_event in lock.iter_mut() {
            if existing_event.deleted {
                continue;
            }
            if existing_event.id == id {
                event_to_update = Some(existing_event);
                continue;
            }
            if existing_event.name == event_data.name {
                return Err(UpdateError::Conflict);
            }
        }

        if let None = event_to_update {
            return Err(UpdateError::NotFound);
        }

        let event = event_to_update.unwrap();

        self.update_event(event, event_data)?;

        Ok(event.clone())
    }

    fn delete(&self, id: u32) -> Result<Event, DeleteError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(DeleteError::Unknown),
        };

        match lock
            .iter_mut()
            .find(|event| event.id == id && !event.deleted)
        {
            Some(event) => {
                event.deleted = true;
                Ok(event.clone())
            }
            None => Err(DeleteError::NotFound),
        }
    }

    fn find_channel(&self, id: u32) -> Result<Channel, FindError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&channel| channel.id == id) {
            Some(channel) => Ok(channel.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindAllError::Unknown),
        };
        Ok(lock.iter().map(|channel| channel.clone()).collect())
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
            .filter_map(|key| existing_users.iter().find(|user| user.id == key))
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
    fn it_should_return_the_event_when_find_is_called_with_an_existing_id() {
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
    fn it_should_return_all_the_events_for_a_given_channel_when_find_all() {
        let repo = InMemoryRepository::new();

        let mock = mocks::mock_event_creation();
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "2";
        mock.channel += "2";
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "3";
        let result = repo.insert(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find_all here ---

        let result = repo.find_all(mocks::mock_channel().name);

        match result {
            Ok(events) => assert_eq!(events.iter().map(|e| e.id).collect::<Vec<u32>>(), vec![0, 2]),
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

    #[test]
    fn it_should_update_event_with_the_provided_data() {
        let repo = InMemoryRepository::new();

        if let Err(..) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created")
        }

        // Testing update here --

        let mut mock = mocks::mock_event_creation();
        mock.name = "Johny".to_string();

        let result = repo.update(0, mock);

        match result {
            Ok(Event { name, .. }) => assert_eq!(name, "Johny"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_conflict_error_when_name_already_exists_while_updating_an_event() {
        let repo = InMemoryRepository::new();

        if let Err(..) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "2";

        if let Err(..) = repo.insert(mock) {
            unreachable!("event must be created")
        }

        // Testing update here --

        let result = repo.update(1, mocks::mock_event_creation());

        match result {
            Err(error) => assert_eq!(error, UpdateError::Conflict),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_not_found_error_when_event_to_update_does_not_exist() {
        let repo = InMemoryRepository::new();

        let result = repo.update(0, mocks::mock_event_creation());

        match result {
            Err(error) => assert_eq!(error, UpdateError::NotFound),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_return_the_list_of_all_channels_on_find_all_channels() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "2";
        mock.channel += "2";
        if let Err(_) = repo.insert(mock) {
            unreachable!("event must be created")
        }

        // Testing find_all_channels here ---

        let result = repo.find_all_channels();

        match result {
            Ok(channels) => assert_eq!(
                channels
                    .iter()
                    .map(|channel| channel.id)
                    .collect::<Vec<u32>>(),
                vec![0, 1]
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_delete_an_event_by_id() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created")
        }

        if let Err(_) = repo.find(0) {
            unreachable!("event must exist")
        }

        // Testing delete here ---

        let result = repo.delete(0);

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find(0) {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!("should not exist"),
        }
    }
}
