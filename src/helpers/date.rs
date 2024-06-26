use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;

use crate::domain::timezone::Timezone;

#[derive(Clone)]
pub struct Date {
    timestamp: i64,
    timezone: Timezone,
}

impl Date {
    pub fn new(timestamp: i64) -> Self {
        return Self {
            timestamp,
            timezone: Timezone::UTC,
        };
    }

    pub fn with_timezone(self: &Self, timezone: Timezone) -> Self {
        return Self {
            timestamp: self.timestamp,
            timezone,
        };
    }

    pub fn timestamp(self: &Self) -> i64 {
        return self.datetime().timestamp();
    }

    pub fn timezone(self: &Self) -> Timezone {
        return self.timezone.clone();
    }

    pub fn to_datetime(self: &Self) -> DateTime<Tz> {
        return self
            .datetime()
            .with_timezone(&Timezone::from(self.timezone.clone()).tz());
    }

    pub fn to_string(self: &Self) -> String {
        return self.to_datetime().to_string();
    }

    fn datetime(self: &Self) -> DateTime<Utc> {
        return DateTime::from_timestamp(self.timestamp.clone(), 0)
            .unwrap_or(DateTime::default())
            .with_second(0)
            .unwrap();
    }
}
