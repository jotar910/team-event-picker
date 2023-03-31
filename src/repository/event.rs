use async_trait::async_trait;
use mongodb::bson::doc;
use serde::de::DeserializeOwned;

use crate::domain::entities::{Channel, Event, EventPick, HasId, User};
use crate::repository::errors::{DeleteError, FindAllError, FindError, InsertError, UpdateError};

#[allow(drop_bounds)]
pub trait Transition: Drop {
    fn commit(&self);
    fn rollback(&self);
}

#[async_trait]
pub trait Repository: Send + Sync {
    async fn find_event(&self, id: u32, channel: u32) -> Result<Event, FindError>;
    async fn find_event_by_name(&self, name: String, channel: u32) -> Result<Event, FindError>;
    async fn find_all_events(&self, channel: u32) -> Result<Vec<Event>, FindAllError>;
    async fn find_all_events_unprotected(&self) -> Result<Vec<Event>, FindAllError>;
    async fn find_all_events_by_id_unprotected(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<Event>, FindAllError>;
    async fn insert_event(&self, event: Event) -> Result<Event, InsertError>;
    async fn update_event(&self, event: Event) -> Result<(), UpdateError>;
    async fn delete_event(&self, id: u32, channel: u32) -> Result<Event, DeleteError>;

    async fn find_channel(&self, id: u32) -> Result<Channel, FindError>;
    async fn find_channel_by_name(&self, name: String) -> Result<Channel, FindError>;
    async fn find_all_channels(&self) -> Result<Vec<Channel>, FindAllError>;
    async fn find_all_channels_by_id(&self, ids: Vec<u32>) -> Result<Vec<Channel>, FindAllError>;
    async fn insert_channel(&self, channel: Channel) -> Result<Channel, InsertError>;

    async fn find_user(&self, id: u32) -> Result<User, FindError>;
    async fn find_users(&self, ids: Vec<u32>) -> Result<Vec<User>, FindAllError>;
    async fn find_users_by_name(&self, name: Vec<String>) -> Result<Vec<User>, FindAllError>;
    async fn insert_users(&self, users: Vec<User>) -> Result<Vec<User>, InsertError>;

    async fn save_pick(&self, pick_data: EventPick) -> Result<(), UpdateError>;
    async fn rev_pick(&self, event_id: u32, channel_id: u32) -> Result<(), UpdateError>;
}

pub struct MongoDbRepository {
    db: mongodb::Database,
}

impl MongoDbRepository {
    pub async fn new(
        uri: &str,
        database: &str,
        pool_size: u32,
    ) -> Result<MongoDbRepository, mongodb::error::Error> {
        // Parse a connection string into an options struct.
        let mut client_options = mongodb::options::ClientOptions::parse(uri).await?;
        client_options.max_pool_size = Some(pool_size);

        let client = mongodb::Client::with_options(client_options)?;
        let db = client.database(database);

        db.run_command(doc! {"ping": 1}, None).await?;

        Ok(MongoDbRepository { db })
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

    async fn find_events_by_name(
        &self,
        name: String,
        channel: u32,
    ) -> Result<Vec<Event>, FindAllError> {
        let filter = doc! { "name": name, "channel": channel, "deleted": false };
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
}

#[async_trait]
impl Repository for MongoDbRepository {
    async fn find_event(&self, id: u32, channel: u32) -> Result<Event, FindError> {
        let filter = doc! { "id": id, "channel": channel, "deleted": false };
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

    async fn find_all_events(&self, channel: u32) -> Result<Vec<Event>, FindAllError> {
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

    async fn find_all_events_unprotected(&self) -> Result<Vec<Event>, FindAllError> {
        let filter = doc! { "deleted": false };
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

    async fn find_all_events_by_id_unprotected(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<Event>, FindAllError> {
        let filter = doc! { "id": { "$in": ids.iter().map(|id| bson::Bson::from(*id)).collect::<Vec<bson::Bson>>() }, "deleted": false };
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
            .find_events_by_name(event.name.clone(), event.channel.clone())
            .await
        {
            Ok(events) if events.len() > 1 || events.len() == 1 && events[0].id != event.id => {
                return Err(UpdateError::Conflict)
            }
            Err(..) => return Err(UpdateError::Unknown),
            _ => (),
        };

        let filter = doc! {"id": event.id};
        let update = doc! {"$set": bson::to_document(&event)?};
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.matched_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }

    async fn delete_event(&self, id: u32, channel: u32) -> Result<Event, DeleteError> {
        let collection = self.db.collection::<Event>("events");

        let filter = doc! { "id": id, "channel": channel, "deleted": false };
        let update = doc! {"$set": {"deleted": true}};
        let result = collection.update_one(filter, update, None).await?;

        if result.matched_count == 0 {
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

    async fn find_all_channels_by_id(&self, ids: Vec<u32>) -> Result<Vec<Channel>, FindAllError> {
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
            .collection::<Channel>("channels")
            .find(filter, None)
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
        let filter = doc! {"id": pick_data.event, "deleted": false};
        let update =
            doc! {"$set": {"prev_pick": pick_data.prev_pick, "cur_pick": pick_data.cur_pick}};
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.matched_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }

    async fn rev_pick(&self, event_id: u32, channel_id: u32) -> Result<(), UpdateError> {
        let event = match self.find_event(event_id, channel_id).await {
            Ok(event) => event,
            Err(error) => {
                return Err(match error {
                    FindError::NotFound => UpdateError::NotFound,
                    FindError::Unknown => UpdateError::Unknown,
                })
            }
        };

        let filter = doc! {"id": event_id, "channel": channel_id, "deleted": false};
        let update = doc! {"$set": { "cur_pick": event.prev_pick }};
        let result = self
            .db
            .collection::<Event>("events")
            .update_one(filter, update, None)
            .await?;

        if result.matched_count == 0 {
            return Err(UpdateError::NotFound);
        }

        Ok(())
    }
}
