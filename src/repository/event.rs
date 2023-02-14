use std::collections::HashSet;
use std::sync::Mutex;
use std::{collections::HashMap, sync::MutexGuard};

use crate::domain::entities::{Channel, Event, EventPick, User};

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
    fn find_by_name(&self, name: String) -> Result<Event, FindError>;
    fn find_all(&self, channel: String) -> Result<Vec<Event>, FindAllError>;
    fn delete(&self, id: u32) -> Result<Event, DeleteError>;

    fn insert_event(&self, event: Event) -> Result<Event, InsertError>;
    fn update_event(&self, event: Event) -> Result<(), UpdateError>;

    fn find_channel(&self, id: u32) -> Result<Channel, FindError>;
    fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError>;
    fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError>;
    fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError>;

    fn find_user(&self, id: u32) -> Result<User, FindError>;
    fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError>;
    fn find_users_by_name(&self, name: Vec<String>) -> Result<Vec<User>, FindAllError>;
    fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError>;

    fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError>;
    fn rev_pick(&self, event_id: u32) -> Result<(), UpdateError>;
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
        match lock.iter().find(|&event| event.id == id && !event.deleted) {
            Some(event) => Ok(event.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    fn find_by_name(&self, name: String) -> Result<Event, FindError> {
        let lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock
            .iter()
            .find(|&event| event.name == name && !event.deleted)
        {
            Some(event) => Ok(event.clone()),
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

    fn insert_event(&self, event: Event) -> Result<Event, InsertError> {
        match self.find_by_name(event.name.clone()) {
            Ok(..) => return Err(InsertError::Conflict),
            Err(error) if error != FindError::NotFound => return Err(InsertError::Unknown),
            _ => (),
        };

        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        let mut event = event.clone();
        event.id = lock.len() as u32;

        lock.push(event.clone());

        Ok(event)
    }

    fn update_event(&self, event: Event) -> Result<(), UpdateError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(UpdateError::Unknown),
        };

        let mut event_to_update: Option<&mut Event> = None;

        for existing_event in lock.iter_mut() {
            if existing_event.deleted {
                continue;
            }
            if existing_event.id == event.id {
                event_to_update = Some(existing_event);
                continue;
            }
            if existing_event.name == event.name {
                return Err(UpdateError::Conflict);
            }
        }

        if let None = event_to_update {
            return Err(UpdateError::NotFound);
        }

        let event_to_update = event_to_update.unwrap();

        *event_to_update = event;

        Ok(())
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

    fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&channel| channel.name == name) {
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

    fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError> {
        let mut lock: MutexGuard<Vec<Channel>> = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        if let Some(..) = lock.iter().find(|&c| c.name == channel.name) {
            return Err(InsertError::Conflict);
        }

        let channel = Channel {
            id: lock.len() as u32,
            name: channel.name,
        };

        lock.push(channel.clone());

        Ok(channel)
    }

    fn find_user(&self, id: u32) -> Result<User, FindError> {
        let lock = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&user| user.id == id) {
            Some(channel) => Ok(channel.clone()),
            _ => Err(FindError::NotFound),
        }
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

    fn find_users_by_name(&self, names: Vec<String>) -> Result<Vec<User>, FindAllError> {
        let lock = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(FindAllError::Unknown),
        };

        let names_set: HashSet<&String> = names.iter().collect();

        let existing_users: Vec<User> = lock
            .iter()
            .filter(|user| names_set.contains(&user.name))
            .map(|user| user.clone())
            .collect();

        let users = names
            .into_iter()
            .filter_map(|key| existing_users.iter().find(|user| user.name == key))
            .cloned()
            .collect();

        Ok(users)
    }

    fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError> {
        let mut lock = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(InsertError::Unknown),
        };

        let users_by_name_map: HashMap<&String, &User> =
            users.iter().map(|user| (&user.name, user)).collect();
        for existing_user in lock.iter() {
            if users_by_name_map.contains_key(&existing_user.name) {
                return Err(InsertError::Conflict);
            }
        }

        let start_id = lock.len() as u32;
        let mut added_users: Vec<User> = vec![];
        for user in users.into_iter() {
            let user = User {
                id: start_id + added_users.len() as u32,
                name: user.name,
            };
            added_users.push(user.clone());
            lock.push(user);
        }

        Ok(added_users)
    }

    fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            Err(..) => return Err(UpdateError::Unknown),
        };

        match lock
            .iter_mut()
            .find(|event| event.id == pick_data.event && !event.deleted)
        {
            Some(event) => {
                event.prev_pick = event.cur_pick;
                event.cur_pick = pick_data.pick;
                Ok(())
            }
            _ => Err(UpdateError::NotFound),
        }
    }

    fn rev_pick(&self, event_id: u32) -> Result<(), UpdateError> {
        let mut lock = match self.events.lock() {
            Ok(lock) => lock,
            Err(..) => return Err(UpdateError::Unknown),
        };

        match lock
            .iter_mut()
            .find(|event| event.id == event_id && !event.deleted)
        {
            Some(event) => {
                event.cur_pick = event.prev_pick;
                Ok(())
            }
            _ => Err(UpdateError::NotFound),
        }
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

        let mock = mocks::mock_event();
        let result = repo.insert_event(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += " 2";
        let result = repo.insert_event(mock);

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

        if let Err(_) = repo.insert_channel(mocks::mock_channel()) {
            unreachable!("channel must be created")
        }

        if let Err(_) = repo.insert_channel(Channel {
            id: 1,
            name: mocks::mock_channel().name + "2",
        }) {
            unreachable!("channel must be created")
        }

        let mock = mocks::mock_event();
        let result = repo.insert_event(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += "2";
        mock.channel = 1;
        let result = repo.insert_event(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += "3";
        let result = repo.insert_event(mock);

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find_all here ---

        let result = repo.find_all(mocks::mock_channel().name);

        match result {
            Ok(events) => assert_eq!(
                events.iter().map(|e| e.id).collect::<Vec<u32>>(),
                vec![0, 2]
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn it_should_find_participants_that_have_the_same_ids_as_the_provided() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert_users(vec![
            User {
                id: 0,
                name: "João".to_string(),
            },
            User {
                id: 0,
                name: "Joana".to_string(),
            },
            User {
                id: 0,
                name: "Francisca".to_string(),
            },
            User {
                id: 0,
                name: "Simão".to_string(),
            },
        ]) {
            unreachable!("users must be created")
        }

        // Testing find_participants here ---

        let result = repo.find_users(vec![1, 2]);

        match result {
            Ok(users) => assert_eq!(
                users,
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
    fn it_should_return_the_list_of_all_channels_on_find_all_channels() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert_channel(mocks::mock_channel()) {
            unreachable!("channel must be created")
        }

        if let Err(_) = repo.insert_channel(Channel {
            id: 1,
            name: mocks::mock_channel().name + "2",
        }) {
            unreachable!("channel must be created")
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

        if let Err(_) = repo.insert_event(mocks::mock_event()) {
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
