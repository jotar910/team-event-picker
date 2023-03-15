use crate::domain::{entities::RepeatPeriod, timezone::Timezone};

pub struct EventSchedule {
    pub id: u32,
    pub timestamp: i64,
    pub timezone: Timezone,
    pub repeat: RepeatPeriod
}