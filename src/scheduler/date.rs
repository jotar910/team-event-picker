use std::{
    ops::{Add, Div, Mul, Sub},
    vec,
};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};

use crate::domain::entities::RepeatPeriod;

use super::helpers;

const MINUTES_IN_A_DAY: i64 = 24 * 60;
const MINUTES_IN_A_WEEK: i64 = 7 * MINUTES_IN_A_DAY;

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
}

struct ChronoUtils();

impl DateUtils for ChronoUtils {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct Date {
    time: DateTime<Utc>,
    frequency: RepeatPeriod,
    utils: Box<dyn DateUtils>,
}

impl Date {
    pub fn new(date: String, repeat: RepeatPeriod) -> Self {
        Self::new_date(date, repeat, Box::new(ChronoUtils()))
    }

    fn new_date(date: String, frequency: RepeatPeriod, utils: Box<dyn DateUtils>) -> Self {
        let time =
            match Utc.datetime_from_str(&date.trim_end_matches(" UTC"), "%Y-%m-%d %H:%M:%S%.3f") {
                Ok(time) => time,
                Err(err) => {
                    log::error!("could not parse date {}: {}", date, err);
                    return Self {
                        time: DateTime::default(),
                        frequency: RepeatPeriod::None,
                        utils,
                    };
                }
            };
        Self {
            time,
            frequency,
            utils,
        }
    }

    pub fn find_minutes(&self) -> Vec<i64> {
        let total = Milliseconds::from(Minutes(helpers::find_ending_minute()));
        let time = Milliseconds::from_timestamp(self.time.timestamp());
        match self.frequency {
            RepeatPeriod::None => {
                let year_start = Milliseconds::from_timestamp(
                    helpers::find_first_day_of_year_timestamp(self.time.year()),
                );
                if self.time.year() == self.utils.now().year() {
                    vec![Minutes::from(time - year_start).0]
                } else {
                    vec![]
                }
            }
            RepeatPeriod::Daily => {
                let interval = Milliseconds::from(Minutes(MINUTES_IN_A_DAY));
                self.find_minutes_by_interval(total, time, interval)
            }
            RepeatPeriod::Weekly(n) => {
                let interval = Milliseconds::from(Minutes((n as i64) * MINUTES_IN_A_WEEK));
                self.find_minutes_by_interval(total, time, interval)
            }
            RepeatPeriod::Monthly(n) => {
                self.find_minutes_by_week_day(n as u32, self.find_week_day())
            }
            RepeatPeriod::Yearly => {
                let year_start = Milliseconds::from_timestamp(
                    helpers::find_first_day_of_year_timestamp(self.time.year()),
                );
                vec![Minutes::from(time - year_start).0]
            }
        }
    }

    fn find_minutes_by_interval(
        &self,
        total: Milliseconds,
        time: Milliseconds,
        interval: Milliseconds,
    ) -> Vec<i64> {
        let year_start = Milliseconds::from_timestamp(helpers::find_first_day_of_year_timestamp(
            self.time.year(),
        ));

        let range_start = time - year_start;
        let range = total - range_start;
        let repetitions = range / interval;

        let mut minutes = vec![];
        for i in 0..repetitions + 1 {
            let millis = range_start + interval * i;
            let date = NaiveDateTime::from_timestamp_millis(time.0 + millis.0).unwrap();
            let weekday = date.weekday();
            if weekday != Weekday::Sat && weekday != Weekday::Sun {
                minutes.push(Minutes::from(millis).0);
            }
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
                .timestamp(),
        );

        let year = today.year();
        let mut month = self.time.month();
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

            let millis =
                Milliseconds::from_timestamp(target_day.and_time(self.time.time()).timestamp())
                    - year_start;
            let minute = Minutes::from(millis);
            minutes.push(minute.0);
            month += monthly_interval;
        }
        minutes
    }

    fn find_week_day(&self) -> (i64, i64) {
        let date = self.time;

        let weekday = self.time.weekday();
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

    #[test]
    fn it_should_create_date_instance() {
        let date = String::from("2001-01-01 01:01:00.000 UTC");
        let repeat = RepeatPeriod::Daily;

        let result = Date::new(date, repeat);
        assert_eq!(
            result.time.date_naive(),
            NaiveDate::from_ymd_opt(2001, 1, 1).unwrap()
        );
        assert_eq!(
            result.time.time(),
            NaiveTime::from_hms_milli_opt(1, 1, 0, 0).unwrap()
        );
        assert_eq!(result.frequency, RepeatPeriod::Daily);
    }

    #[test]
    fn it_should_return_no_minutes_when_frequency_is_none_and_year_is_different() {
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::None;

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::from_ymd(2000, 1, 1)));
        let result = result.find_minutes();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn it_should_return_the_corresponding_minutes_when_frequency_is_none_and_year_is_same() {
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::None;

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::from_ymd(2023, 1, 1)));
        let result = result.find_minutes();
        assert_eq!(result, vec![MINUTES_IN_A_DAY + 1]);
    }

    #[test]
    fn it_should_return_the_corresponding_minutes_when_frequency_is_yearly() {
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Yearly;

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::from_ymd(2023, 1, 1)));
        let result = result.find_minutes();
        assert_eq!(result, vec![MINUTES_IN_A_DAY + 1]);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_daily_frequency_until_end_of_the_year() {
        let date = String::from("2023-01-01 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Daily;

        let result = Date::new(date, repeat);
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
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Weekly(1);

        let result = Date::new(date, repeat);
        let result = result.find_minutes();
        assert_eq!(result.len(), 52);

        let minutes: Vec<i64> = (0..52)
            .into_iter()
            .map(|index| 2 + index * 7)
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_biweekly_frequency_until_end_of_the_year() {
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Weekly(2);

        let result = Date::new(date, repeat);
        let result = result.find_minutes();
        assert_eq!(result.len(), 26);

        let minutes: Vec<i64> = (0..26)
            .into_iter()
            .map(|index| 2 + index * 7 * 2)
            .map(|day| (day - 1) * (24 * 60) + 1)
            .collect();
        assert_eq!(result, minutes);
    }

    #[test]
    fn it_should_return_all_the_minutes_for_monthly_frequency_until_end_of_the_year() {
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Monthly(1);

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::new()));
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
        let date = String::from("2023-01-31 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Monthly(1);

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::new()));
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
        let date = String::from("2023-01-02 00:01:00.000 UTC");
        let repeat = RepeatPeriod::Monthly(2);

        let result = Date::new_date(date, repeat, Box::new(MockDateUtils::new()));
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
                now_date: DateTime::from_utc(
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
    }
}
