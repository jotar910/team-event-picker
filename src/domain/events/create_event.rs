use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_trim::{string_trim, vec_string_trim};

use crate::domain::entities::{Event, RepeatPeriod};
use crate::domain::timezone::Timezone;
use crate::repository::errors::{FindError, InsertError};
use crate::repository::event::Repository;

#[derive(Deserialize, Clone, Debug)]
pub struct Request {
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    pub timestamp: i64,
    pub timezone: String,
    pub repeat: String,
    #[serde(deserialize_with = "vec_string_trim")]
    pub participants: Vec<String>,
    #[serde(skip_deserializing)]
    pub channel: String,
    #[serde(skip_deserializing)]
    pub team_id: String,
    #[serde(skip_deserializing)]
    pub max_events: u32,
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub id: u32,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadRequest,
    Forbidden,
    Conflict,
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    validate_channels_count(repo.clone(), req.channel.clone(), req.team_id.clone(), req.max_events).await?;

    match repo
        .clone()
        .find_event_by_name(req.name.clone(), req.channel.clone())
        .await
    {
        Ok(..) => {
            log::trace!(
                "could not add event with name {} on channel {}: event already exists",
                req.name,
                req.channel
            );
            return Err(Error::Conflict);
        }
        Err(error) if error != FindError::NotFound => return Err(Error::Unknown),
        _ => (),
    };

    let mut event = Event {
        id: 0,
        name: req.name.clone(),
        timestamp: req.timestamp,
        timezone: Timezone::from(req.timezone.clone()),
        repeat: RepeatPeriod::try_from(req.repeat.clone()).map_err(|err| {
            log::trace!("could not parse repeat period {}: {:?}", req.repeat, err);
            Error::BadRequest
        })?,
        participants: vec![],
        channel: req.channel,
        team_id: req.team_id.clone(),
        deleted: false,
    };
    event.participants = req
        .participants
        .into_iter()
        .map(|user| user.into())
        .collect();

    match repo.insert_event(event).await {
        Ok(Event {
               id,
               timestamp,
               timezone,
               repeat,
               ..
           }) => Ok(Response {
            id,
            timestamp,
            timezone,
            repeat,
        }),
        Err(err) => Err(match err {
            InsertError::Conflict => Error::Conflict,
            InsertError::Unknown => Error::Unknown,
        }),
    }
}

async fn validate_channels_count(
    repo: Arc<dyn Repository>,
    channel: String,
    team_id: String,
    max_events: u32,
) -> Result<(), Error> {
    if is_special(team_id.clone()) {
        log::trace!("skipping channels count validation for special team {}", team_id);
        return Ok(());
    }
    let count = repo.count_events(channel.clone()).await.map_err(|err| {
        log::error!("counting events for channel {} failed: {:?}", channel, err);
        Error::Unknown
    })?;
    if count == max_events {
        log::warn!(
            "could not add more events on channel {}: max channels {} reached",
            channel,
            max_events
        );
        return Err(Error::Forbidden);
    }
    Ok(())
}

fn is_special(team_id: String) -> bool {
    std::env::var("SPECIAL_TEAM_ID")
        .inspect_err(|err| log::warn!("could not read special team id: {:?}", err))
        .map_or(false, |special| {
            log::debug!("create_event: special team id: {}", special);
            special == team_id
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_special_true() {
        std::env::set_var("SPECIAL_TEAM_ID", "special");
        assert_eq!(is_special("special".to_string()), true);
    }

    #[test]
    fn is_special_false() {
        std::env::set_var("SPECIAL_TEAM_ID", "special");
        assert_eq!(is_special("not_special".to_string()), false);
    }
}
