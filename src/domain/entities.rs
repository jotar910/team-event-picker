use super::timezone::Timezone;
use crate::helpers::date::Date;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

pub trait HasId {
    fn set_id(&mut self, id: u32);
    fn get_id(&self) -> u32;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Event {
    pub id: u32,
    pub name: String,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub participants: Vec<Participant>,
    pub channel: String,
    pub team_id: String,
    pub deleted: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OldEvent {
    pub id: u32,
    pub name: String,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod,
    pub participants: Vec<u32>,
    pub channel: u32,
    pub prev_pick: u32,
    pub cur_pick: u32,
    pub team_id: String,
    pub deleted: bool,
}

impl Event {
    pub fn migrate(
        old: OldEvent,
        users: &HashMap<u32, String>,
        channels: &HashMap<u32, String>,
    ) -> Self {
        let channel = channels.get(&old.channel).unwrap().clone();
        Self {
            id: old.id,
            name: old.name,
            timestamp: old.timestamp,
            timezone: old.timezone,
            repeat: old.repeat,
            participants: old
                .participants
                .into_iter()
                .enumerate()
                .map(|(i, p)| {
                    let user = users.get(&p).unwrap().clone();
                    Participant {
                        user,
                        picked: picked(old.cur_pick, i),
                        created_at: old.timestamp,
                        picked_at: picked_at(old.cur_pick, old.prev_pick, i),
                    }
                })
                .collect(),
            channel,
            team_id: old.team_id,
            deleted: old.deleted,
        }
    }
}

fn picked(cur_pick: u32, index: usize) -> bool {
    if index >= 32 {
        return false;
    }
    return cur_pick & (1 << index) != 0;
}

fn was_last_picked(cur_pick: u32, prev_pick: u32, index: usize) -> bool {
    return picked(cur_pick, index) && !picked(prev_pick, index);
}

fn picked_at(cur_pick: u32, prev_pick: u32, index: usize) -> Option<i64> {
    if was_last_picked(cur_pick, prev_pick, index) {
        return Some(Date::now().timestamp());
    }
    if picked(cur_pick, index) {
        return Some(Date::now().timestamp() - 1);
    }
    return None;
}

impl HasId for OldEvent {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

impl HasId for Event {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Participant {
    pub user: String,
    pub picked: bool,
    pub created_at: i64,
    pub picked_at: Option<i64>,
}

impl From<String> for Participant {
    fn from(user: String) -> Self {
        Self {
            user,
            picked: false,
            created_at: Date::now().timestamp(),
            picked_at: None,
        }
    }
}

pub struct EventPick {
    pub event: u32,
    pub cur_pick: u32,
    pub prev_pick: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Channel {
    pub id: u32,
    pub name: String,
}

impl HasId for Channel {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct User {
    pub id: u32,
    pub name: String,
}

impl HasId for User {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum RepeatPeriod {
    None,
    Daily,
    Weekly(i32),
    Monthly(i32),
    Yearly,
}

impl RepeatPeriod {
    pub fn label(&self) -> String {
        match self {
            RepeatPeriod::Daily => "Daily",
            RepeatPeriod::Weekly(1) => "Weekly",
            RepeatPeriod::Weekly(2) => "Bi-weekly",
            RepeatPeriod::Monthly(1) => "Monthly",
            RepeatPeriod::Monthly(2) => "Bi-monthly",
            RepeatPeriod::Yearly => "Yearly",
            _ => "None",
        }
        .to_string()
    }

    pub fn value(label: String) -> RepeatPeriod {
        log::error!("label: {}", label);
        match label.as_str() {
            "Daily" => RepeatPeriod::Daily,
            "Weekly" => RepeatPeriod::Weekly(1),
            "Bi-weekly" => RepeatPeriod::Weekly(2),
            "Monthly" => RepeatPeriod::Monthly(1),
            "Bi-monthly" => RepeatPeriod::Monthly(2),
            "Yearly" => RepeatPeriod::Yearly,
            _ => RepeatPeriod::None,
        }
    }
}

impl TryFrom<String> for RepeatPeriod {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "none" => Ok(RepeatPeriod::None),
            "daily" => Ok(RepeatPeriod::Daily),
            "weekly" => Ok(RepeatPeriod::Weekly(1)),
            "weekly_two" => Ok(RepeatPeriod::Weekly(2)),
            "monthly" => Ok(RepeatPeriod::Monthly(1)),
            "monthly_two" => Ok(RepeatPeriod::Monthly(2)),
            "yearly" => Ok(RepeatPeriod::Yearly),
            _ => Err(()),
        }
    }
}

impl TryFrom<RepeatPeriod> for String {
    type Error = String;

    fn try_from(value: RepeatPeriod) -> Result<Self, Self::Error> {
        Ok(match value {
            RepeatPeriod::None => "none",
            RepeatPeriod::Daily => "daily",
            RepeatPeriod::Weekly(1) => "weekly",
            RepeatPeriod::Weekly(2) => "weekly_two",
            RepeatPeriod::Monthly(1) => "monthly",
            RepeatPeriod::Monthly(2) => "monthly_two",
            RepeatPeriod::Yearly => "yearly",
            _ => return Err(format!("Invalid RepeatPeriod: {:?}", value)),
        }
        .to_string())
    }
}

impl Display for RepeatPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Auth {
    pub id: u32,
    pub team: String,
    pub access_token: String,
    pub deleted: bool,
}

impl HasId for Auth {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

impl Display for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "team={}, access_token={}, deleted={}",
            self.team, self.access_token, self.deleted
        )
    }
}
