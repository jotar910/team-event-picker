use std::sync::Arc;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::Channel;
use crate::repository::errors::FindAllError;
use crate::repository::event::Repository;

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>) -> Result<ListResponse<Channel>, Error> {
    match repo.find_all_channels().await {
        Err(err) => {
            return match err {
                FindAllError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(channels) => Ok(ListResponse::new(channels)),
    }
}
