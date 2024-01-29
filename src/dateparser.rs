use chrono::{DateTime, Datelike, Days, Month, NaiveTime, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;
use regex::Regex;

use std::ops::Sub;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidHighlevelStructure(String),
    InvalidDay(String),
    InvalidTime(String),
    InvalidMonth(String),
    InvalidRelativeDay(String),
    ArithmeticProblem,
}

pub type ParseResult<T> = Result<T, ParseError>;

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
        _ => Err(ParseError::InvalidMonth(month_short_name.to_string())),
    }
}

fn parse_hh_mm(time: &str) -> ParseResult<NaiveTime> {
    NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| ParseError::InvalidTime(time.to_string()))
}

fn parse_day(day: &str) -> ParseResult<u32> {
    match day.parse::<u32>() {
        Ok(d) if d >= 1 && d <= 31 => Ok(d),
        _ => Err(ParseError::InvalidDay(day.to_string())),
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
            _ => Err(ParseError::InvalidRelativeDay(relday_s.to_string())),
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
            .ok_or(ParseError::ArithmeticProblem)
    }

    pub fn parse(&self, ts: &str) -> ParseResult<DateTime<Utc>> {
        if let Some(patts) = REL_TIME.captures(ts) {
            let (_, [relday_s, hhmm_s]) = patts.extract();
            self.parse_rel_time(relday_s, hhmm_s)
        } else if let Some(patts) = ABS_TIME.captures(ts) {
            let (_, [day_s, month_s, hhmm_s]) = patts.extract();
            self.parse_abs_time(day_s, month_s, hhmm_s)
        } else {
            Err(ParseError::InvalidHighlevelStructure(ts.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_month_short() {
        assert_eq!(parse_month_short("tam"), Ok(Month::January));
        assert_eq!(
            parse_month_short("foo"),
            Err(ParseError::InvalidMonth("foo".to_string()))
        );
    }

    #[test]
    fn test_parse_hh_mm() {
        assert_eq!(
            parse_hh_mm("01:23"),
            Ok(NaiveTime::from_hms_opt(1, 23, 0).unwrap())
        );
        assert_eq!(
            parse_hh_mm("01:60"),
            Err(ParseError::InvalidTime("01:60".to_string()))
        );
        assert_eq!(
            parse_hh_mm("25:24"),
            Err(ParseError::InvalidTime("25:24".to_string()))
        );
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

        let result = parser.parse("tänään 25:48");
        assert_eq!(result, Err(ParseError::InvalidTime("25:48".to_string())));
    }

    #[test]
    fn test_parse_ts_absolute() {
        let parser = DateParser::new(get_time());
        let result = parser.parse("21 huh 19:52");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2022, 4, 21, 19, 52, 0).unwrap())
        );
        let result = parser.parse("32 tam 01:32");
        assert_eq!(result, Err(ParseError::InvalidDay("32".to_string())));
    }
}