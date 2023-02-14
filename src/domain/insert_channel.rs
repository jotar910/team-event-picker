use std::sync::Arc;

use crate::repository::event::{FindError, InsertError, Repository};

use crate::domain::entities::Channel;

pub struct Request {
    pub name: String,
}

pub struct Response {
    pub channel: Channel,
}

pub enum Error {
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    match repo.find_channel_by_name(req.name.clone()) {
        Ok(channel) => return Ok(Response { channel }),
        Err(error) if error != FindError::NotFound => return Err(Error::Unknown),
        _ => (),
    }

    let channel = Channel {
        id: 0,
        name: req.name,
    };

    let channel: Channel = repo.insert_channel(channel).map_err(|error| match error {
        InsertError::Conflict | InsertError::Unknown => Error::Unknown,
    })?;

    Ok(Response { channel })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_update_participants_for_the_given_event() {
        let repo = Arc::new(InMemoryRepository::new());

        let req = Request {
            name: mocks::mock_channel().name,
        };

        let result = execute(repo, req);

        match result {
            Ok(Response { channel }) => assert_eq!(channel, mocks::mock_channel()),
            _ => unreachable!(),
        }
    }
}
