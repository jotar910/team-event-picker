use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::repository::event::Repository;

use crate::domain::entities::{Channel, EventPick, User};

use super::helpers;

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
    pub channel_id: u32,
    pub channel_url: String,
    pub user_id: u32,
    pub user_name: String,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    Unknown,
}

pub struct PickResult {
    event_id: u32,
    event_name: String,
    cur_pick: u32,
    prev_pick: u32,
    channel_id: u32,
    user_id: u32,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let events = repo
        .find_all_events_by_id_unprotected(req.events)
        .await
        .unwrap_or(Vec::new());

    let mut channel_ids: HashSet<u32> = HashSet::new();
    let mut user_ids: HashSet<u32> = HashSet::new();
    let event_picks: Vec<PickResult> = events
        .into_iter()
        .map(|event| {
            let (pick, user_id) = helpers::pick(&event);
            let channel_id = event.channel;

            channel_ids.insert(event.channel);
            user_ids.insert(user_id);

            PickResult {
                event_id: event.id,
                event_name: event.name,
                prev_pick: pick.prev_pick,
                cur_pick: pick.cur_pick,
                channel_id,
                user_id,
            }
        })
        .collect();

    let channels: HashMap<u32, Channel> = repo
        .find_all_channels_by_id(channel_ids.into_iter().collect::<Vec<u32>>())
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|channel| (channel.id, channel))
        .collect();

    let users: HashMap<u32, User> = repo
        .find_users(user_ids.into_iter().collect::<Vec<u32>>())
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|user| (user.id, user))
        .collect();

    let mut picks: HashMap<u32, Pick> = HashMap::new();
    for pick in event_picks.into_iter() {
        if !channels.contains_key(&pick.channel_id) || !users.contains_key(&pick.user_id) {
            continue;
        }

        if let Err(err) = repo
            .save_pick(EventPick {
                event: pick.event_id,
                prev_pick: pick.prev_pick,
                cur_pick: pick.cur_pick,
            })
            .await
        {
            log::error!("ignoring pick: could not save event pick: {:?}", err);
            continue;
        }

        picks.insert(
            pick.event_id,
            Pick {
                event_id: pick.event_id,
                event_name: pick.event_name,
                channel_id: pick.channel_id,
                channel_url: channels.get(&pick.channel_id).unwrap().name.clone(),
                user_id: pick.user_id,
                user_name: users.get(&pick.user_id).unwrap().name.clone(),
            },
        );
    }

    Ok(Response { picks })
}