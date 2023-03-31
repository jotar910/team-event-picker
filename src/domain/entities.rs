use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::timezone::Timezone;

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
    pub participants: Vec<u32>,
    pub channel: u32,
    pub prev_pick: u32,
    pub cur_pick: u32,
    pub team_id: String,
    pub deleted: bool,
}

impl HasId for Event {
    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
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
    type Error = ();

    fn try_from(value: RepeatPeriod) -> Result<Self, Self::Error> {
        Ok(match value {
            RepeatPeriod::None => "none",
            RepeatPeriod::Daily => "daily",
            RepeatPeriod::Weekly(1) => "weekly",
            RepeatPeriod::Weekly(2) => "weekly_two",
            RepeatPeriod::Monthly(1) => "monthly",
            RepeatPeriod::Monthly(2) => "monthly_two",
            RepeatPeriod::Yearly => "yearly",
            _ => return Err(()),
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
