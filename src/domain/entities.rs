use serde::{Deserialize, Serialize};

pub trait HasId {
    fn set_id(&mut self, id: u32);
    fn get_id(&self) -> u32;
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Event {
    pub id: u32,
    pub name: String,
    pub date: String,
    pub repeat: RepeatPeriod,
    pub participants: Vec<u32>,
    pub channel: u32,
    pub prev_pick: u32,
    pub cur_pick: u32,
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
    pub pick: u32,
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
    Monthly,
    Yearly,
}

impl TryFrom<String> for RepeatPeriod {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "none" => Ok(RepeatPeriod::None),
            "daily" => Ok(RepeatPeriod::Daily),
            "weekly" => Ok(RepeatPeriod::Weekly(1)),
            "weekly_two" => Ok(RepeatPeriod::Weekly(2)),
            "monthly" => Ok(RepeatPeriod::Monthly),
            "yearly" => Ok(RepeatPeriod::Yearly),
            _ => Err(()),
        }
    }
}
