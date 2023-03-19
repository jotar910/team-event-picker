use async_trait::async_trait;
use bson::doc;

use crate::domain::entities::{Auth, HasId};

use super::errors::{self, FindError, InsertError, UpdateError};

#[async_trait]
pub trait Repository: Send + Sync {
    async fn insert(&self, auth: Auth) -> Result<Auth, InsertError>;
    async fn update(&self, auth: Auth) -> Result<Auth, UpdateError>;
    async fn find_by_team(&self, team: String) -> Result<Auth, FindError>;
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
}

impl MongoDbRepository {
    async fn fill_with_id<'a, T>(
        collection: &'a mongodb::Collection<T>,
        value: &'a mut T,
    ) -> Result<&'a mut T, mongodb::error::Error>
    where
        T: HasId + serde::de::DeserializeOwned + Unpin + Send + Sync,
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
}

#[async_trait]
impl Repository for MongoDbRepository {
    async fn insert(&self, auth: Auth) -> Result<Auth, errors::InsertError> {
        match self.find_by_team(auth.team.clone()).await {
            Ok(..) => return Err(InsertError::Conflict),
            Err(error) if error != FindError::NotFound => return Err(InsertError::Unknown),
            _ => (),
        };

        let mut result = auth.clone();
        let collection = self.db.collection::<Auth>("tokens");

        collection
            .insert_one(Self::fill_with_id(&collection, &mut result).await?, None)
            .await?;

        Ok(result)
    }

    async fn update(&self, auth: Auth) -> Result<Auth, errors::UpdateError> {
        let filter = doc! {"id": auth.id};
        let update = doc! {"$set": bson::to_document(&auth)?};
        let result = self
            .db
            .collection::<Auth>("tokens")
            .update_one(filter, update, None)
            .await?;

        if result.matched_count == 0 {
            return Err(UpdateError::NotFound);
        }
        Ok(auth)
    }

    async fn find_by_team(&self, team: String) -> Result<Auth, errors::FindError> {
        let filter = doc! { "team": team, "deleted": false };
        let cursor = self
            .db
            .collection::<Auth>("tokens")
            .find_one(filter, None)
            .await?;

        match cursor {
            Some(event) => Ok(event),
            None => Err(FindError::NotFound),
        }
    }
}
