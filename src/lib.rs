use chrono::{DateTime, Datelike, Days, Month, NaiveTime, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;
use regex::Regex;

use std::ops::Sub;

#[derive(Debug, PartialEq)]
pub enum ParseErrorKind {
    InvalidDay,
    InvalidMonth,
    InvalidTime,
    InvalidRelativeDay,
    UnknownFormat,
}

pub type ParseResult<T> = Result<T, ParseErrorKind>;

fn parse_month_short(short_name: &str) -> ParseResult<Month> {
    match short_name {
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
        _ => Err(ParseErrorKind::InvalidMonth),
    }
}
fn parse_hh_mm(time: &str) -> ParseResult<NaiveTime> {
    NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| ParseErrorKind::InvalidTime)
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

    fn parse_rel_time(&self, ts: &str) -> ParseResult<DateTime<Utc>> {
        match REL_TIME.captures(ts) {
            Some(patts) => {
                let (_, [relday_s, hhmm_s]) = patts.extract();
                let hhmm = parse_hh_mm(hhmm_s)?;
                let offset = match relday_s {
                    "tänään" => Ok(Days::new(0)),
                    "eilen" => Ok(Days::new(1)),
                    _ => Err(ParseErrorKind::InvalidRelativeDay),
                }?;
                let date = self.server_time.date_naive().sub(offset);
                let tz = self.server_time.offset();
                let new_ts: chrono::LocalResult<DateTime<Utc>> = tz.with_ymd_and_hms(
                    date.year(),
                    date.month(),
                    date.day(),
                    hhmm.hour(),
                    hhmm.minute(),
                    0,
                );
                Ok(new_ts.unwrap())
            }
            None => Err(ParseErrorKind::UnknownFormat),
        }
    }

    fn parse_abs_time(&self, ts: &str) -> ParseResult<DateTime<Utc>> {
        // regex error
        // int parse error
        // chrono parse
        match ABS_TIME.captures(ts) {
            Some(patts) => {
                let (_, [day_s, month_s, hhmm_s]) = patts.extract();
                let day_num = day_s
                    .parse::<u32>()
                    .map_err(|_| ParseErrorKind::InvalidDay)?;
                let month = parse_month_short(month_s)?;
                let hhmm = parse_hh_mm(hhmm_s)?;
                let new_ts = self
                    .server_time
                    .offset()
                    .with_ymd_and_hms(
                        self.server_time.year(),
                        month.number_from_month(),
                        day_num,
                        hhmm.hour(),
                        hhmm.minute(),
                        0,
                    )
                    .unwrap();
                if new_ts > self.server_time {
                    let x = new_ts
                        .with_year(new_ts.year() - 1)
                        .ok_or(ParseErrorKind::UnknownFormat)?;
                    Ok(x)
                } else {
                    Ok(new_ts)
                }
            }
            None => Err(ParseErrorKind::UnknownFormat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_month_short() {
        assert_eq!(parse_month_short("tam"), Ok(Month::January));
        assert_eq!(parse_month_short("foo"), Err(ParseErrorKind::InvalidMonth));
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
        let result = parser.parse_rel_time("tänään 01:23");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2023, 3, 25, 1, 23, 0).unwrap())
        );

        let result = parser.parse_rel_time("eilen 15:59");
        assert_eq!(
            result,
            Ok(Utc.with_ymd_and_hms(2023, 3, 24, 15, 59, 0).unwrap())
        );
    }

    #[test]
    fn test_parse_ts_absolute() {
        let parser = DateParser::new(get_time());
        let result = parser.parse_abs_time("21 huh 19:52").unwrap();
        assert_eq!(
            result,
            Utc.with_ymd_and_hms(2022, 4, 21, 19, 52, 0).unwrap()
        );
    }
}
