use async_trait::async_trait;
use mongodb::bson::doc;
use serde::de::DeserializeOwned;

use crate::domain::entities::{Event, HasId};
use crate::repository::errors::{
    CountError, DeleteError, FindAllError, FindError, InsertError, UpdateError,
};

#[async_trait]
pub trait Repository: Send + Sync {
    async fn find_event(&self, id: u32, channel: String) -> Result<Event, FindError>;
    async fn find_event_by_name(&self, name: String, channel: String) -> Result<Event, FindError>;
    async fn find_all_events(&self, channel: String) -> Result<Vec<Event>, FindAllError>;
    async fn find_all_events_unprotected(&self) -> Result<Vec<Event>, FindAllError>;
    async fn find_all_events_by_id_unprotected(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<Event>, FindAllError>;
    async fn insert_event(&self, event: Event) -> Result<Event, InsertError>;
    async fn update_event(&self, event: Event) -> Result<(), UpdateError>;
    async fn delete_event(&self, id: u32, channel: String) -> Result<Event, DeleteError>;
    async fn count_events(&self, channel: String) -> Result<u32, CountError>;
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

        Ok(MongoDbRepository {
            db,
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

    async fn find_events_by_name(
        &self,
        name: String,
        channel: String,
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
    async fn find_event(&self, id: u32, channel: String) -> Result<Event, FindError> {
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

    async fn find_event_by_name(&self, name: String, channel: String) -> Result<Event, FindError> {
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
            Ok(..) => {
                log::error!(
                    "insert_event: event with name {} already exists",
                    event.name
                );
                return Err(InsertError::Conflict);
            }
            Err(error) if error != FindError::NotFound => {
                log::error!("insert_event: inserting event failed: {:?}", error);
                return Err(InsertError::Unknown);
            }
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

    async fn delete_event(&self, id: u32, channel: String) -> Result<Event, DeleteError> {
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

    async fn count_events(&self, channel: String) -> Result<u32, CountError> {
        let filter = doc! { "channel": channel, "deleted": false };
        let count = self
            .db
            .collection::<Event>("events")
            .count_documents(filter, None)
            .await?;

        Ok(count as u32)
    }
}

#[cfg(test)]
mod test {
    use log::LevelFilter;

    use super::*;

    #[tokio::test]
    async fn test_migration() {
        let db_tool_url =
            std::env::var("DATABASE_TOOL_URL").expect("DATABASE_TOOL_URL must be set");
        let db_tool_name =
            std::env::var("DATABASE_TOOL_NAME").expect("DATABASE_TOOL_NAME must be set");
        let repository = MongoDbRepository::new(&db_tool_url, &db_tool_name, 10)
            .await
            .unwrap();
        tracing_subscriber::fmt::init();
        log::set_max_level(LevelFilter::Trace);
        assert!(repository
            .migrate()
            .await
            .map_err(|err| {
                log::error!("Error migrating: {:?}", err);
                err
            })
            .is_ok());
    }

    #[tokio::test]
    async fn test_copy() {
        let from_db_tool_url =
            std::env::var("FROM_DATABASE_TOOL_URL").expect("FROM_DATABASE_TOOL_URL must be set");
        let from_db_tool_name =
            std::env::var("FROM_DATABASE_TOOL_NAME").expect("FROM_DATABASE_TOOL_NAME must be set");
        let from_repository = MongoDbRepository::new(&from_db_tool_url, &from_db_tool_name, 10)
            .await
            .unwrap();
        let to_db_tool_url =
            std::env::var("TO_DATABASE_TOOL_URL").expect("TO_DATABASE_TOOL_URL must be set");
        let to_db_tool_name =
            std::env::var("TO_DATABASE_TOOL_NAME").expect("TO_DATABASE_TOOL_NAME must be set");
        let to_repository = MongoDbRepository::new(&to_db_tool_url, &to_db_tool_name, 10)
            .await
            .unwrap();
        tracing_subscriber::fmt::init();
        log::set_max_level(LevelFilter::Trace);
        assert!(to_repository
            .copy::<Channel>(&from_repository, "channels")
            .await
            .map_err(|err| {
                log::error!("Error copying: {:?}", err);
                err
            })
            .is_ok());
    }
}
