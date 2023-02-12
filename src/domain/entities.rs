#[derive(Clone)]
pub struct Event {
    pub id: u32,
    pub name: String,
    pub date: String,
    pub repeat: RepeatPeriod,
    pub participants: Vec<u32>,
    pub channel: u32,
    pub deleted: bool,
}

pub struct EventCreation {
    pub name: String,
    pub date: String,
    pub repeat: RepeatPeriod,
    pub participants: Vec<String>,
    pub channel: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
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
