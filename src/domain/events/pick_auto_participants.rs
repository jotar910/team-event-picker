use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::Auth;
use crate::domain::events::pick_participant;
use crate::repository::{auth, event};

pub struct Request {
    pub events: Vec<u32>,
}

#[derive(Debug)]
pub struct Response {
    pub picks: HashMap<u32, Pick>,
}

#[derive(Debug)]
pub struct Pick {
    pub event_id: u32,
    pub event_name: String,
    pub channel_id: String,
    pub user_id: String,
    pub team_id: String,
    pub left_count: usize,
    pub access_token: String,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Unknown,
}

pub async fn execute(
    event_repo: Arc<dyn event::Repository>,
    auth_repo: Arc<dyn auth::Repository>,
    req: Request,
) -> Result<Response, Error> {
    let events = event_repo
        .find_all_events_by_id_unprotected(req.events)
        .await
        .unwrap_or(Vec::new());

    let tokens: HashMap<String, Auth> = auth_repo
        .find_all_by_team(
            events
                .iter()
                .map(|event| event.team_id.clone())
                .collect::<Vec<String>>()
                .drain(..)
                .collect(),
        )
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|auth| (auth.team.clone(), auth))
        .collect();

    let mut picks: HashMap<u32, Pick> = HashMap::new();
    for event in events.iter() {
        let pick = match pick_participant::execute(
            event_repo.clone(),
            pick_participant::Request {
                event: event.id,
                channel: event.channel.clone(),
            },
        )
        .await
        {
            Ok(pick) => pick,
            Err(error) => {
                log::info!(
                    "ignoring pick: no participants for event {}: err {:?}",
                    event.id,
                    error
                );
                continue;
            }
        };

        picks.insert(
            event.id,
            Pick {
                event_id: event.id,
                event_name: event.name.clone(),
                channel_id: event.channel.clone(),
                user_id: pick.id,
                team_id: event.team_id.clone(),
                left_count: event.participants.iter().filter(|pick| !pick.picked).count(),
                access_token: tokens.get(&event.team_id)
                    .and_then(|auth| Some(auth.access_token.clone()))
                    .unwrap_or_else(|| {
                        log::error!("could not find access token for team id {} while picking automatically for the event {}", event.team_id, event.id);
                        String::from("")
                    }),
            },
        );
    }

    Ok(Response { picks })
}
