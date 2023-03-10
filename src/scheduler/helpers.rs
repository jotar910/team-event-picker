use chrono::{Datelike, Duration, NaiveDate, Timelike, Utc};

pub fn sleep_until_next_minute() {
    let now = Utc::now();
    let next_minute = now.with_second(0).unwrap() + Duration::minutes(1);
    let diff_secs = (next_minute.timestamp() as u64) - (now.timestamp() as u64);

    std::thread::sleep(std::time::Duration::from_secs(diff_secs));
}

pub fn find_current_minute() -> i64 {
    let now = Utc::now().with_second(0).unwrap();

    (now.timestamp() - find_first_day_of_year_timestamp(now.year())) / 60
}

pub fn find_ending_minute() -> i64 {
    let now = Utc::now().with_second(0).unwrap();

    (find_first_day_of_year_timestamp(now.year() + 1) - find_first_day_of_year_timestamp(now.year())) / 60 - 1
}

pub fn find_first_day_of_year_timestamp(year: i32) -> i64 {
    NaiveDate::from_ymd_opt(year, 1, 1)
        .unwrap()
        .and_hms_milli_opt(0, 0, 0, 0)
        .unwrap()
        .timestamp()
}
