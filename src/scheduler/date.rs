use std::{
    ops::{Add, Div, Mul, Sub},
    vec,
};

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc, Weekday};

use crate::domain::{entities::RepeatPeriod, timezone::Timezone};
use crate::helpers::date::Date;

use super::helpers;

#[derive(Clone, Copy)]
struct Milliseconds(i64);

#[derive(Clone, Copy)]
struct Minutes(i64);

impl Milliseconds {
    fn from_timestamp(timestamp: i64) -> Self {
        Self(timestamp * 1000)
    }
}

impl From<Milliseconds> for Minutes {
    fn from(value: Milliseconds) -> Self {
        Minutes(value.0 / 60_000)
    }
}

impl From<Minutes> for Milliseconds {
    fn from(value: Minutes) -> Self {
        Milliseconds(value.0 * 60_000)
    }
}

impl Add<Milliseconds> for Milliseconds {
    type Output = Self;

    fn add(self, rhs: Milliseconds) -> Self::Output {
        Milliseconds(self.0 + rhs.0)
    }
}

impl Sub<Milliseconds> for Milliseconds {
    type Output = Self;

    fn sub(self, rhs: Milliseconds) -> Self::Output {
        Milliseconds(self.0 - rhs.0)
    }
}

impl Div<Milliseconds> for Milliseconds {
    type Output = u32;

    fn div(self, rhs: Milliseconds) -> Self::Output {
        (self.0 / rhs.0) as u32
    }
}

impl Mul<u32> for Milliseconds {
    type Output = Milliseconds;

    fn mul(self, rhs: u32) -> Self::Output {
        Milliseconds(self.0 * (rhs as i64))
    }
}

trait DateUtils: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn clone(&self) -> Box<dyn DateUtils>;
}

struct ChronoUtils();

impl DateUtils for ChronoUtils {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn clone(&self) -> Box<dyn DateUtils> {
        Box::new(Self {})
    }
}

pub struct SchedulerDate {
    date: Date,
    frequency: RepeatPeriod,
    utils: Box<dyn DateUtils>,
}

impl SchedulerDate {
    pub fn new(timestamp: i64, timezone: Timezone, repeat: RepeatPeriod) -> Self {
        Self::new_date(timestamp, timezone, repeat, Box::new(ChronoUtils()))
    }

    fn new_date(
        timestamp: i64,
        timezone: Timezone,
        frequency: RepeatPeriod,
        utils: Box<dyn DateUtils>,
    ) -> Self {
        Self {
            date: Date::new(timestamp).with_timezone(timezone),
            frequency,
            utils,
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            date: self.date.clone(),
            frequency: self.frequency.clone(),
            utils: self.utils.clone(),
        }
    }

    pub fn find_minutes(&self) -> Vec<i64> {
        let time = Milliseconds::from_timestamp(self.date.timestamp());
        match self.frequency {
            RepeatPeriod::None => {
                let year_start = Milliseconds::from_timestamp(
                    helpers::find_first_day_of_year_timestamp(self.date.to_datetime().year()),
                );
                if self.date.to_datetime().year() == self.utils.now().year() {
                    vec![Minutes::from(time - year_start).0]
                } else {
                    vec![]
                }
            }
            RepeatPeriod::Daily => self.find_minutes_by_interval(time, 1),
            RepeatPeriod::Weekly(n) => self.find_minutes_by_interval(time, (n as u32) * 7),
            RepeatPeriod::Monthly(n) => {
                self.find_minutes_by_week_day(n as u32, self.find_week_day())
            }
            RepeatPeriod::Yearly => {
                let year_start = Milliseconds::from_timestamp(
                    helpers::find_first_day_of_year_timestamp(self.date.to_datetime().year()),
                );
                vec![Minutes::from(time - year_start).0]
            }
        }
    }

    fn find_minutes_by_interval(&self, time: Milliseconds, interval: u32) -> Vec<i64> {
        let year_start = Milliseconds::from_timestamp(helpers::find_first_day_of_year_timestamp(
            self.date.to_datetime().year(),
        ));
        let year_end = Milliseconds::from_timestamp(helpers::find_first_day_of_year_timestamp(
            self.date.to_datetime().year() + 1,
        ));
        let interval_duration = Duration::days(interval as i64);

        let mut position_time = time;
        let mut minutes = vec![];
        while position_time.0 < year_end.0 {
            let position_date = DateTime::from_timestamp_millis(position_time.0).unwrap();
            let position_weekday = position_date.weekday();
            if interval != 1
                || (position_weekday != Weekday::Sat && position_weekday != Weekday::Sun)
            {
                let position = Milliseconds::from_timestamp(
                    self.date
                        .timezone()
                        .tz()
                        .from_local_datetime(&position_date.naive_local())
                        .unwrap()
                        .timestamp(),
                ) - year_start;
                minutes.push(Minutes::from(position).0);
            }
            let next_position_date = position_date + interval_duration;
            position_time = Milliseconds::from_timestamp(next_position_date.timestamp());
        }

        minutes
    }

    fn find_minutes_by_week_day(
        &self,
        monthly_interval: u32,
        (num_days_from_monday, week_number_of_month): (i64, i64),
    ) -> Vec<i64> {
        let today = self.utils.now();
        let year_start = Milliseconds::from_timestamp(
            NaiveDate::from_ymd_opt(today.year(), 1, 1)
                .unwrap()
                .and_hms_milli_opt(0, 0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp(),
        );

        let year = today.year();
        let mut month = self.date.to_datetime().month();
        let mut minutes = vec![];

        while month <= 12 {
            let first_day_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            let first_weekday = first_day_of_month.weekday();
            let diff_week_days_from_monday =
                (num_days_from_monday + 7 - (first_weekday.num_days_from_monday() as i64)) % 7;

            let diff_days_from_first_day_of_month =
                (7 * week_number_of_month + diff_week_days_from_monday) as i64;
            let mut target_day =
                first_day_of_month + Duration::days(diff_days_from_first_day_of_month);

            let target_month = target_day.month();
            let target_year = target_day.year();
            if target_month < month && target_year == year || target_year < year {
                target_day = target_day + Duration::days(7);
            } else if target_month > month && target_year == year || target_year > year {
                target_day = target_day - Duration::days(7);
            }

            let millis = Milliseconds::from_timestamp(
                target_day.and_time(self.date.to_datetime().time()).and_utc().timestamp(),
            ) - year_start;
            let minute = Minutes::from(millis);
            minutes.push(minute.0);
            month += monthly_interval;
        }
        minutes
    }

    fn find_week_day(&self) -> (i64, i64) {
        let date = self.date.to_datetime();

        let weekday = self.date.to_datetime().weekday();
        let num_days_from_monday = weekday.num_days_from_monday() as i64;

        let first_day_of_month =
            NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date.date_naive());
        let first_weekday = first_day_of_month.weekday();
        let days_before_first_weekday =
            (weekday.num_days_from_monday() + 7 - first_weekday.num_days_from_monday()) % 7;
        let week_number_of_month = ((date.day() - days_before_first_weekday + 6) / 7) as i64;

        (num_days_from_monday, week_number_of_month - 1)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;

    const MINUTES_IN_A_DAY: i64 = 24 * 60;

    #[test]
    fn it_should_create_date_instance() {
        let date = 978310860; // String::from("2001-01-01 01:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Daily;

        let result = SchedulerDate::new(date, timezone, repeat);
        assert_eq!(
            result.date.to_datetime().date_naive(),
            NaiveDate::from_ymd_opt(2001, 1, 1).unwrap()
        );
        assert_eq!(
            result.date.to_datetime().time(),
            NaiveTime::from_hms_milli_opt(1, 1, 0, 0).unwrap()
        );
        assert_eq!(result.frequency, RepeatPeriod::Daily);
    }

    #[test]
    fn it_should_return_no_minutes_when_frequency_is_none_and_year_is_different() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::None;

        let result = SchedulerDate::new_date(
            date,
            timezone,
            repeat,
            Box::new(MockDateUtils::from_ymd(2000, 1, 1)),
        );
        let result = result.find_minutes();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn it_should_return_the_corresponding_minutes_when_frequency_is_none_and_year_is_same() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::None;

        let result = SchedulerDate::new_date(
            date,
            timezone,
            repeat,
            Box::new(MockDateUtils::from_ymd(2023, 1, 1)),
        );
        let result = result.find_minutes();
        assert_eq!(result, vec![MINUTES_IN_A_DAY + 1]);
    }

    #[test]
    fn it_should_return_the_corresponding_minutes_when_frequency_is_yearly() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Yearly;

        let result = SchedulerDate::new_date(
            date,
            timezone,
            repeat,
            Box::new(MockDateUtils::from_ymd(2023, 1, 1)),
        );
        let result = result.find_minutes();
        assert_eq!(result, vec![MINUTES_IN_A_DAY + 1]);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_daily_frequency_until_end_of_the_year() {
        let date = 1672531260; // String::from("2023-01-01 00:01:00.000 UTC");
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Daily;

        let result = SchedulerDate::new(date, timezone, repeat);
        let result = result.find_minutes();
        assert_eq!(result.len(), 260);

        let minutes: Vec<i64> = vec![2..7, 9..14, 16..21, 23..28, 30..32]
            .into_iter()
            .flat_map(|range| range.collect::<Vec<i64>>())
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert!(minutes.len() > 0 && minutes[0] == 1441);
        assert_eq!(result[..minutes.len()], minutes);

        let minutes: Vec<i64> = vec![1..2, 4..9, 11..16, 18..23, 25..30]
            .into_iter()
            .flat_map(|range| range.collect::<Vec<i64>>())
            .map(|day| (day - 1) * (24 * 60) + 1440 * (365 - 31) + 1)
            .collect();
        assert!(minutes.len() > 0 && minutes[0] == (1440 * (365 - 31) + 1));
        assert_eq!(result[result.len() - minutes.len()..], minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_weekly_frequency_until_end_of_the_year() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Weekly(1);

        let result = SchedulerDate::new(date, timezone, repeat);
        let result = result.find_minutes();
        assert_eq!(result.len(), 52);

        let minutes: Vec<i64> = (0..52)
            .into_iter()
            .map(|index| 2 + index * 7)
            .enumerate()
            .map(|(index, day)| {
                (day - 1) * (24 * 60) + 1 - (if index < 12 || index > 42 { 0 } else { 60 })
            })
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_biweekly_frequency_until_end_of_the_year() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Weekly(2);

        let result = SchedulerDate::new(date, timezone, repeat);
        let result = result.find_minutes();
        assert_eq!(result.len(), 26);

        let minutes: Vec<i64> = (0..26)
            .into_iter()
            .map(|index| 2 + index * 7 * 2)
            .enumerate()
            .map(|(index, day)| {
                (day - 1) * (24 * 60) + 1 - (if index < 6 || index > 21 { 0 } else { 60 })
            })
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_monthly_frequency_until_end_of_the_year() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Monthly(1);

        let result =
            SchedulerDate::new_date(date, timezone, repeat, Box::new(MockDateUtils::new()));
        let result = result.find_minutes();
        assert_eq!(result.len(), 12);

        let days = vec![2, 6, 6, 3, 1, 5, 3, 7, 4, 2, 6, 4];
        let months = vec![0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
        let minutes: Vec<i64> = days
            .into_iter()
            .enumerate()
            .map(|(index, day)| day + months[..index + 1].iter().sum::<i64>())
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_monthly_frequency_in_last_month_day_until_end_of_the_year(
    ) {
        let date = 1675123260; // String::from("2023-01-31 00:01:00.000 UTC");
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Monthly(1);

        let result =
            SchedulerDate::new_date(date, timezone, repeat, Box::new(MockDateUtils::new()));
        let result = result.find_minutes();
        assert_eq!(result.len(), 12);

        let days = vec![31, 28, 28, 25, 30, 27, 25, 29, 26, 31, 28, 26];
        let months = vec![0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
        let minutes: Vec<i64> = days
            .into_iter()
            .enumerate()
            .map(|(index, day)| day + months[..index + 1].iter().sum::<i64>())
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_bimonthly_frequency_until_end_of_the_year() {
        let date = 1672617660; // String::from("2023-01-02 00:01:00.000 UTC")
        let timezone = Timezone::UTC;
        let repeat = RepeatPeriod::Monthly(2);

        let result =
            SchedulerDate::new_date(date, timezone, repeat, Box::new(MockDateUtils::new()));
        let result = result.find_minutes();
        assert_eq!(result.len(), 6);

        let days = vec![2, 6, 1, 3, 4, 6];
        let months = vec![0, 31 + 28, 31 + 30, 31 + 30, 31 + 31, 30 + 31, 30];
        let minutes: Vec<i64> = days
            .into_iter()
            .enumerate()
            .map(|(index, day)| day + months[..index + 1].iter().sum::<i64>())
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert_eq!(result, minutes);
    }

    struct MockDateUtils {
        now_date: DateTime<Utc>,
    }

    impl MockDateUtils {
        fn new() -> Self {
            Self::from_ymd(2023, 3, 9)
        }

        fn from_ymd(year: i32, month: u32, day: u32) -> Self {
            Self {
                now_date: DateTime::from_naive_utc_and_offset(
                    NaiveDate::from_ymd_opt(year, month, day)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    Utc,
                ),
            }
        }
    }

    impl DateUtils for MockDateUtils {
        fn now(&self) -> DateTime<Utc> {
            self.now_date
        }

        fn clone(&self) -> Box<dyn DateUtils> {
            Box::new(Self {
                now_date: self.now_date.clone(),
            })
        }
    }
}
