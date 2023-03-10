use crate::domain::entities::RepeatPeriod;

pub struct EventSchedule {
    pub id: u32,
    pub date: String,
    pub repeat: RepeatPeriod
}