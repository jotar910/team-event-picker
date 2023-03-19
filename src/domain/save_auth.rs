use std::sync::Arc;

use crate::repository::{
    auth::Repository,
    errors::{FindError, InsertError, UpdateError},
};

use super::entities::Auth;

pub struct Request {
    pub team: String,
    pub access_token: String,
}

impl From<Request> for Auth {
    fn from(value: Request) -> Self {
        Self {
            id: 0,
            team: value.team,
            access_token: value.access_token,
            deleted: false,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Conflict,
    Unknown,
}

impl From<InsertError> for Error {
    fn from(value: InsertError) -> Self {
        match value {
            InsertError::Conflict => Error::Conflict,
            InsertError::Unknown => Error::Unknown,
        }
    }
}

impl From<UpdateError> for Error {
    fn from(value: UpdateError) -> Self {
        match value {
            UpdateError::Conflict => Error::Conflict,
            UpdateError::NotFound | UpdateError::Unknown => Error::Unknown,
        }
    }
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Auth, Error> {
    let result = match repo.clone().find_by_team(req.team.clone()).await {
        Ok(Auth { id, .. }) => repo.update(Auth { id, ..req.into() }).await?,
        Err(err) if err == FindError::NotFound => repo.insert(req.into()).await?,
        Err(..) => return Err(Error::Unknown),
    };

    Ok(result)
}
