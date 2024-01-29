use chrono::{DateTime, Datelike, Days, Month, NaiveTime, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;
use regex::Regex;

use std::ops::Sub;

pub type ParseResult<T> = Result<T, String>;

fn parse_month_short(month_short_name: &str) -> ParseResult<Month> {
    match month_short_name {
        "tam" => Ok(Month::January),
        "hel" => Ok(Month::February),
        "maa" => Ok(Month::March),
        "huh" => Ok(Month::April),
        "tou" => Ok(Month::May),
        "kes" => Ok(Month::June),
        "hei" => Ok(Month::July),
        "elo" => Ok(Month::August),
        "syy" => Ok(Month::September),
        "lok" => Ok(Month::October),
        "mar" => Ok(Month::November),
        "jou" => Ok(Month::December),
        _ => Err(format!("unknown month: '{month_short_name}'")),
    }
}

fn parse_hh_mm(time: &str) -> ParseResult<NaiveTime> {
    NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| format!("invalid time format: '{time}'"))
}

fn parse_day(day: &str) -> ParseResult<u32> {
    match day.parse::<u32>() {
        Ok(d) if d >= 1 && d <= 31 => Ok(d),
        _ => Err(format!("invalid day number: '{day}'")),
    }
}

pub struct DateParser {
    server_time: DateTime<Utc>,
}

lazy_static! {
    static ref REL_TIME: Regex = Regex::new(r"\s*(eilen|tänään)\s+(\d{2}:\d{2})\s*").unwrap();
    static ref ABS_TIME: Regex =
        Regex::new(r"\s*(\d{1,2})\s+([a-zA-Z]{3})\s+(\d{2}:\d{2})\s*").unwrap();
}

impl DateParser {
    pub fn new(server_time: DateTime<Utc>) -> Self {
        DateParser {
            server_time: server_time,
        }
    }

    fn parse_rel_time(&self, relday_s: &str, hhmm_s: &str) -> ParseResult<DateTime<Utc>> {
        let hhmm = parse_hh_mm(hhmm_s)?;
        let day_offset = match relday_s {
            "tänään" => Ok(Days::new(0)),
            "eilen" => Ok(Days::new(1)),
            _ => Err(format!("uknown relative day token: '{relday_s}'")),
        }?;
        let date = self.server_time.date_naive().sub(day_offset);
        let new_ts = self.server_time.offset().with_ymd_and_hms(
            date.year(),
            date.month(),
            date.day(),
            hhmm.hour(),
            hhmm.minute(),
            0,
        );
        Ok(new_ts.unwrap())
    }

    fn parse_abs_time(
        &self,
        day_s: &str,
        month_s: &str,
        hhmm_s: &str,
    ) -> ParseResult<DateTime<Utc>> {
        let day = parse_day(day_s)?;
        let month = parse_month_short(month_s)?.number_from_month();
        let hhmm = parse_hh_mm(hhmm_s)?;

        let new_ts = self
            .server_time
            .offset()
            .with_ymd_and_hms(
                self.server_time.year(),
                month,
                day,
                hhmm.hour(),
                hhmm.minute(),
                0,
            )
            .unwrap();

        // timestamp can be in the future; check manually since we lack the actual year.
        // this assumes no item can be listed for over a year.
        let y_offset = if new_ts > self.server_time { 1 } else { 0 };

        new_ts
            .with_year(new_ts.year() - y_offset)
            .ok_or("error calculating timestamp in the past".to_string())
    }

    pub fn parse(&self, ts: &str) -> ParseResult<DateTime<Utc>> {
        if let Some(patts) = REL_TIME.captures(ts) {
            let (_, [relday_s, hhmm_s]) = patts.extract();
            self.parse_rel_time(relday_s, hhmm_s)
        } else if let Some(patts) = ABS_TIME.captures(ts) {
            let (_, [day_s, month_s, hhmm_s]) = patts.extract();
            self.parse_abs_time(day_s, month_s, hhmm_s)
        } else {
            Err(format!("unrecognized timestamp format: '{ts}'"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_month_short() {
        assert_eq!(parse_month_short("tam"), Ok(Month::January));
        assert!(parse_month_short("foo").is_err());
    }

    #[test]
    fn test_parse_hh_mm() {
        assert_eq!(
            parse_hh_mm("01:23"),
            Ok(NaiveTime::from_hms_opt(1, 23, 0).unwrap())
        );
        assert!(parse_hh_mm("01:60").is_err());
        assert!(parse_hh_mm("25:24").is_err());
    }

    fn get_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2023, 3, 25, 10, 52, 1).unwrap()
    }

    #[test]
    fn test_parse_ts_relative() {
        let parser = DateParser::new(get_time());
        let result = parser.parse("tänään 01:23");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2023, 3, 25, 1, 23, 0).unwrap())
        );

        let result = parser.parse("eilen 15:59");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2023, 3, 24, 15, 59, 0).unwrap())
        );
    }

    #[test]
    fn test_parse_ts_absolute() {
        let parser = DateParser::new(get_time());
        let result = parser.parse("21 huh 19:52");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2022, 4, 21, 19, 52, 0).unwrap())
        );
    }
}
