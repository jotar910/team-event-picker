use std::sync::Arc;

use crate::repository::{auth::Repository, errors::FindError};

use super::entities::Auth;

pub struct Request {
    pub team: String,
}

#[derive(Debug)]
pub enum Error {
    Unauthorized,
    Unknown,
}

impl From<FindError> for Error {
    fn from(value: FindError) -> Self {
        match value {
            FindError::NotFound => Error::Unauthorized,
            FindError::Unknown => Error::Unknown,
        }
    }
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Auth, Error> {
    Ok(repo.clone().find_by_team(req.team.clone()).await?)
}
