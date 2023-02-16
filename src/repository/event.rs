use std::collections::HashSet;
use std::sync::Mutex;
use std::{collections::HashMap, sync::MutexGuard};

use async_trait::async_trait;
use mongodb::bson::doc;
use serde::de::DeserializeOwned;

use crate::domain::entities::{Channel, Event, EventPick, HasId, User};

#[derive(Debug, PartialEq)]
pub enum FindError {
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for FindError {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

pub enum FindAllError {
    Unknown,
}

impl From<mongodb::error::Error> for FindAllError {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InsertError {
    Conflict,
    Unknown,
}

impl From<mongodb::error::Error> for InsertError {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UpdateError {
    Conflict,
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for UpdateError {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

impl From<bson::ser::Error> for UpdateError {
    fn from(value: bson::ser::Error) -> Self {
        match value {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DeleteError {
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for DeleteError {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[allow(drop_bounds)]
pub trait Transition: Drop {
    fn commit(&self);
    fn rollback(&self);
}

#[async_trait]
pub trait Repository: Send + Sync {
    fn transition(&self) -> Box<dyn Transition>;

    async fn find_event(&self, id: u32) -> Result<Event, FindError>;
    async fn find_event_by_name(&self, name: String, channel: u32) -> Result<Event, FindError>;
    async fn find_all_events(&self, channel: String) -> Result<Vec<Event>, FindAllError>;
    async fn insert_event(&self, event: Event) -> Result<Event, InsertError>;
    async fn update_event(&self, event: Event) -> Result<(), UpdateError>;
    async fn delete_event(&self, id: u32) -> Result<Event, DeleteError>;

    async fn find_channel(&self, id: u32) -> Result<Channel, FindError>;
    async fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError>;
    async fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError>;
    async fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError>;

    async fn find_user(&self, id: u32) -> Result<User, FindError>;
    async fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError>;
    async fn find_users_by_name(&self, name: Vec<String>) -> Result<Vec<User>, FindAllError>;
    async fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError>;

    async fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError>;
    async fn rev_pick(&self, event_id: u32) -> Result<(), UpdateError>;
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

#[async_trait]
impl Repository for InMemoryRepository {
    fn transition(&self) -> Box<dyn Transition> {
        Box::new(InMemoryTransaction::new())
    }

    async fn find_event(&self, id: u32) -> Result<Event, FindError> {
        let lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&event| event.id == id && !event.deleted) {
            Some(event) => Ok(event.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    async fn find_event_by_name(&self, name: String, channel: u32) -> Result<Event, FindError> {
        let lock = match self.events.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock
            .iter()
            .find(|&event| event.name == name && event.channel == channel && !event.deleted)
        {
            Some(event) => Ok(event.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    async fn find_all_events(&self, channel: String) -> Result<Vec<Event>, FindAllError> {
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

    async fn insert_event(&self, event: Event) -> Result<Event, InsertError> {
        match self
            .find_event_by_name(event.name.clone(), event.channel.clone())
            .await
        {
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

    async fn update_event(&self, event: Event) -> Result<(), UpdateError> {
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

    async fn delete_event(&self, id: u32) -> Result<Event, DeleteError> {
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

    async fn find_channel(&self, id: u32) -> Result<Channel, FindError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&channel| channel.id == id) {
            Some(channel) => Ok(channel.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    async fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&channel| channel.name == name) {
            Some(channel) => Ok(channel.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    async fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError> {
        let lock = match self.channels.lock() {
            Ok(lock) => lock,
            _ => return Err(FindAllError::Unknown),
        };
        Ok(lock.iter().map(|channel| channel.clone()).collect())
    }

    async fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError> {
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

    async fn find_user(&self, id: u32) -> Result<User, FindError> {
        let lock = match self.users.lock() {
            Ok(lock) => lock,
            _ => return Err(FindError::Unknown),
        };
        match lock.iter().find(|&user| user.id == id) {
            Some(channel) => Ok(channel.clone()),
            _ => Err(FindError::NotFound),
        }
    }

    async fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError> {
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

    async fn find_users_by_name(&self, names: Vec<String>) -> Result<Vec<User>, FindAllError> {
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

    async fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError> {
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

    async fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError> {
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

    async fn rev_pick(&self, event_id: u32) -> Result<(), UpdateError> {
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

pub struct MongoDbRepository {
    db: mongodb::Database,
}

impl MongoDbRepository {
    pub async fn new(
        uri: &str,
        database: &str,
    ) -> Result<MongoDbRepository, mongodb::error::Error> {
        // Parse a connection string into an options struct.
        let client_options = mongodb::options::ClientOptions::parse(uri).await?;

        // Get a handle to the deployment.
        let client = mongodb::Client::with_options(client_options)?;

        Ok(MongoDbRepository {
            db: client.database(database),
        })
    }

    async fn fill_with_id<'a, T>(
        collection: &'a mongodb::Collection<T>,
        value: &'a mut T,
    ) -> Result<&'a mut T, mongodb::error::Error>
    where
        T: HasId + DeserializeOwned + Unpin + Send + Sync,
    {
        let options = mongodb::options::FindOneOptions::builder()
            .sort(doc! { "id": -1 })
            .build();

        // Get the highest ID in the collection
        let highest_id = match collection.find_one(None, options).await? {
            Some(result) => result.get_id(),
            None => 0,
        };

        // Assign the next available ID to the event
        value.set_id(highest_id + 1);

        Ok(value)
    }

    async fn fill_with_ids<'a, T>(
        collection: &'a mongodb::Collection<T>,
        values: &'a mut Vec<T>,
    ) -> Result<&'a mut Vec<T>, mongodb::error::Error>
    where
        T: HasId + DeserializeOwned + Unpin + Send + Sync,
    {
        let options = mongodb::options::FindOneOptions::builder()
            .sort(doc! { "id": -1 })
            .build();

        // Get the highest ID in the collection
        let highest_id = match collection.find_one(None, options).await? {
            Some(result) => result.get_id(),
            None => 0,
        };

        // Assign the next available ID to the event
        values
            .iter_mut()
            .enumerate()
            .for_each(|(i, event)| event.set_id(highest_id + 1 + (i as u32)));

        Ok(values)
    }
}

#[async_trait]
impl Repository for MongoDbRepository {
    fn transition(&self) -> Box<dyn Transition> {
        Box::new(InMemoryTransaction::new())
    }

    async fn find_event(&self, id: u32) -> Result<Event, FindError> {
        let filter = doc! { "id": id, "deleted": false };
        let cursor = self
            .db
            .collection::<Event>("events")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(event) => Ok(event),
            None => Err(FindError::NotFound),
        }
    }

    async fn find_event_by_name(&self, name: String, channel: u32) -> Result<Event, FindError> {
        let filter = doc! { "name": name, "channel": channel, "deleted": false };
        let cursor = self
            .db
            .collection::<Event>("events")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(event) => Ok(event),
            None => Err(FindError::NotFound),
        }
    }

    async fn find_all_events(&self, channel: String) -> Result<Vec<Event>, FindAllError> {
        let filter = doc! { "channel": channel, "deleted": false };
        let mut cursor = self
            .db
            .collection::<Event>("events")
            .find(filter, None)
            .await?;

        let mut result: Vec<Event> = vec![];
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(result)
    }

    async fn insert_event(&self, event: Event) -> Result<Event, InsertError> {
        match self
            .find_event_by_name(event.name.clone(), event.channel.clone())
            .await
        {
            Ok(..) => return Err(InsertError::Conflict),
            Err(error) if error != FindError::NotFound => return Err(InsertError::Unknown),
            _ => (),
        };

        let mut result = event.clone();
        let collection = self.db.collection::<Event>("events");

        collection
            .insert_one(Self::fill_with_id(&collection, &mut result).await?, None)
            .await?;

        Ok(result)
    }

    async fn update_event(&self, event: Event) -> Result<(), UpdateError> {
        match self
            .find_event_by_name(event.name.clone(), event.channel.clone())
            .await
        {
            Ok(..) => return Err(UpdateError::Conflict),
            Err(error) if error != FindError::NotFound => return Err(UpdateError::Unknown),
            _ => (),
        };

        let filter = doc! {"id": event.id};
        let update = bson::to_document(&event)?;
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.modified_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }

    async fn delete_event(&self, id: u32) -> Result<Event, DeleteError> {
        let collection = self.db.collection::<Event>("events");

        let filter = doc! { "id": id, "deleted": false };
        let update = doc! {"$set": {"deleted": true}};
        let result = collection.update_one(filter, update, None).await?;

        if result.modified_count == 0 {
            return Err(DeleteError::NotFound);
        }

        let filter = doc! { "id": id, "deleted": true };
        let cursor = collection.find_one(filter, None).await?;

        match cursor {
            Some(event) => Ok(event),
            None => Err(DeleteError::NotFound),
        }
    }

    async fn find_channel(&self, id: u32) -> Result<Channel, FindError> {
        let filter = doc! { "id": id };
        let cursor = self
            .db
            .collection::<Channel>("channels")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(channel) => Ok(channel),
            None => Err(FindError::NotFound),
        }
    }

    async fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError> {
        let filter = doc! { "name": name };
        let cursor = self
            .db
            .collection::<Channel>("channels")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(channel) => Ok(channel),
            None => Err(FindError::NotFound),
        }
    }

    async fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError> {
        let mut cursor = self
            .db
            .collection::<Channel>("channels")
            .find(None, None)
            .await?;

        let mut result: Vec<Channel> = vec![];
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(result)
    }

    async fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError> {
        match self.find_channel_by_name(channel.name.clone()).await {
            Ok(..) => return Err(InsertError::Conflict),
            Err(error) if error != FindError::NotFound => return Err(InsertError::Unknown),
            _ => (),
        };

        let mut result = channel.clone();
        let collection = self.db.collection::<Channel>("channels");

        collection
            .insert_one(Self::fill_with_id(&collection, &mut result).await?, None)
            .await?;

        Ok(result)
    }

    async fn find_user(&self, id: u32) -> Result<User, FindError> {
        let filter = doc! { "id": id };
        let cursor = self
            .db
            .collection::<User>("users")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(user) => Ok(user),
            None => Err(FindError::NotFound),
        }
    }

    async fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError> {
        let filter = doc! {
            "id": {
                "$in": ids
                    .iter()
                    .map(|id| bson::Bson::from(*id))
                    .collect::<Vec<bson::Bson>>()
            }
        };
        let mut cursor = self
            .db
            .collection::<User>("users")
            .find(filter, None)
            .await?;

        let mut result: Vec<User> = vec![];
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(result)
    }

    async fn find_users_by_name(&self, names: Vec<String>) -> Result<Vec<User>, FindAllError> {
        let filter = doc! {
            "name": {
                "$in": names
                    .iter()
                    .map(|name| bson::Bson::from(name))
                    .collect::<Vec<bson::Bson>>()
            }
        };
        let mut cursor = self
            .db
            .collection::<User>("users")
            .find(filter, None)
            .await?;

        let mut result: Vec<User> = vec![];
        while cursor.advance().await? {
            result.push(cursor.deserialize_current()?);
        }
        Ok(result)
    }

    async fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError> {
        match self
            .find_users_by_name(users.iter().map(|user| user.name.clone()).collect())
            .await
        {
            Ok(users) if users.len() > 0 => return Err(InsertError::Conflict),
            Err(..) => return Err(InsertError::Unknown),
            _ => (),
        };

        let mut result = users.clone();
        let collection = self.db.collection::<User>("users");

        collection
            .insert_many(Self::fill_with_ids(&collection, &mut result).await?, None)
            .await?;

        Ok(result)
    }

    async fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError> {
        let event = match self.find_event(pick_data.event.clone()).await {
            Ok(event) => event,
            Err(error) => {
                return Err(match error {
                    FindError::NotFound => UpdateError::NotFound,
                    FindError::Unknown => UpdateError::Unknown,
                })
            }
        };

        let filter = doc! {"id": pick_data.event, "deleted": false};
        let update = doc! {"$set": { "prev_pick": event.cur_pick, "cur_pick": pick_data.pick }};
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.modified_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }

    async fn rev_pick(&self, event_id: u32) -> Result<(), UpdateError> {
        let event = match self.find_event(event_id.clone()).await {
            Ok(event) => event,
            Err(error) => {
                return Err(match error {
                    FindError::NotFound => UpdateError::NotFound,
                    FindError::Unknown => UpdateError::Unknown,
                })
            }
        };

        let filter = doc! {"id": event_id, "deleted": false};
        let update = doc! {"$set": { "cur_pick": event.prev_pick }};
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.modified_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;

    #[tokio::test]
    async fn it_should_return_not_found_error_when_find_event_does_not_exist() {
        let repo = InMemoryRepository::new();

        let result = repo.find_event(0).await;

        match result {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn it_should_return_the_event_when_find_is_called_with_an_existing_id() {
        let repo = InMemoryRepository::new();

        let mock = mocks::mock_event();
        let result = repo.insert_event(mock).await;

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += " 2";
        let result = repo.insert_event(mock).await;

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find here ---

        let result = repo.find_event(1).await;

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 1),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn it_should_return_all_the_events_for_a_given_channel_when_find_all() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert_channel(mocks::mock_channel()).await {
            unreachable!("channel must be created")
        }

        if let Err(_) = repo.insert_channel(Channel {
            id: 1,
            name: mocks::mock_channel().name + "2",
        }).await {
            unreachable!("channel must be created")
        }

        let mock = mocks::mock_event();
        let result = repo.insert_event(mock).await;

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += "2";
        mock.channel = 1;
        let result = repo.insert_event(mock).await;

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        let mut mock = mocks::mock_event();
        mock.name += "3";
        let result = repo.insert_event(mock).await;

        if let Err(_) = result {
            unreachable!("event must be created")
        }

        // Testing find_all here ---

        let result = repo.find_all_events(mocks::mock_channel().name).await;

        match result {
            Ok(events) => assert_eq!(
                events.iter().map(|e| e.id).collect::<Vec<u32>>(),
                vec![0, 2]
            ),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn it_should_find_participants_that_have_the_same_ids_as_the_provided() {
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
        ]).await {
            unreachable!("users must be created")
        }

        // Testing find_participants here ---

        let result = repo.find_users(vec![1, 2]).await;

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

    #[tokio::test]
    async fn it_should_return_the_list_of_all_channels_on_find_all_channels() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert_channel(mocks::mock_channel()).await {
            unreachable!("channel must be created")
        }

        if let Err(_) = repo.insert_channel(Channel {
            id: 1,
            name: mocks::mock_channel().name + "2",
        }).await {
            unreachable!("channel must be created")
        }

        // Testing find_all_channels here ---

        let result = repo.find_all_channels().await;

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

    #[tokio::test]
    async fn it_should_delete_an_event_by_id() {
        let repo = InMemoryRepository::new();

        if let Err(_) = repo.insert_event(mocks::mock_event()).await {
            unreachable!("event must be created")
        }

        if let Err(_) = repo.find_event(0).await {
            unreachable!("event must exist")
        }

        // Testing delete here ---

        let result = repo.delete_event(0).await;

        match result {
            Ok(Event { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find_event(0).await {
            Err(err) => assert_eq!(err, FindError::NotFound),
            _ => unreachable!("should not exist"),
        }
    }
}
