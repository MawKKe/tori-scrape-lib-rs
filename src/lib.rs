use std::fs;
use std::path::Path;

use chrono::{DateTime, Datelike, Days, LocalResult, Month, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use encoding_rs::ISO_8859_15;
use encoding_rs_io::DecodeReaderBytesBuilder;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::Html;
use scraper::Selector;
use std::io::BufReader;
use std::io::Read;
use std::ops::Sub;

#[derive(Debug)]
pub struct Item {
    pub id: String,
    pub company_ad: bool,
    pub href: String,
    pub price: Option<Price>,
    pub img: String,
    pub title: String,
    pub posted_at_orig: String,
    pub posted_at: DateTime<Utc>,
    pub location: String,
    pub direction: String,
    pub seller: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ItemParseError {
    Success,
    MissingID,
    MissingTitle,
    MissingHref,
    MissingCompanyAd,
    MissingImg,
    MissingPostedAt,
    MissingLocation,
    MissingDirection,
    UnexpectedValue,
    NotEnoughItems,
    InvalidPrice(String),
}

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

type ItemParseResult<T> = Result<T, ItemParseError>;

pub struct Parser {
    server_time: DateTime<Utc>,
    user_today: DateTime<chrono_tz::Tz>,
    user_yesterday: DateTime<chrono_tz::Tz>,
}

pub fn parse_file(path: &Path) {
    let file = fs::File::open(path).unwrap();
    let transcoded = DecodeReaderBytesBuilder::new()
        .encoding(Some(ISO_8859_15))
        .build(file);

    let mut reader = BufReader::new(transcoded);
    let mut buf = String::new();
    let n = reader.read_to_string(&mut buf).unwrap();
    assert!(n > 0);
    let doc = Html::parse_document(&buf);
    let parser = Parser::new(chrono_tz::Europe::Helsinki, Utc::now());
    let result = parser.parse_document(&doc);
    match result {
        Ok(items) => {
            for itm in items {
                println!("{:#?}", itm);
            }
        }
        Err(e) => {
            println!("could not parse items: {:?}", e);
        }
    }
}

fn reformat_ws(input: &str) -> String {
    let w = input.split_whitespace();
    w.collect::<Vec<&str>>().join(" ")
}

fn remove_prefix_maybe(prefix: &str, input: &str) -> String {
    input.strip_prefix(prefix).unwrap_or(input).to_string()
}

#[derive(Debug, PartialEq)]
pub struct Price {
    pub value: i32,
    pub unit: String,
}

lazy_static! {
    static ref PRICE_PATT: Regex = Regex::new(r"\s*([0-9][0-9\s]*)\s+(€)\s*").unwrap();
    static ref ROW_SELECTOR: Selector = Selector::parse("a[data-row]").unwrap();
    static ref TITLE_SELECTOR: Selector = Selector::parse("div .li-title").unwrap();
    static ref PRICE_SELECTOR: Selector = Selector::parse("p .list_price, .ineuros").unwrap();
    static ref IMAGE_SELECTOR: Selector = Selector::parse("div .item_image[src]").unwrap();
    static ref POSTED_AT_SELECTOR: Selector = Selector::parse("div .date_image").unwrap();
    static ref COMBINED_SELECTOR: Selector = Selector::parse("div .cat_geo > p").unwrap();
    static ref REL_TIME: Regex = Regex::new(r"\s*(eilen|tänään)\s+(\d{2}:\d{2})\s*").unwrap();
    static ref ABS_TIME: Regex =
        Regex::new(r"\s*(\d{1,2})\s+([a-zA-Z]{3})\s+(\d{2}:\d{2})\s*").unwrap();
}

fn price_parse(input: &str) -> ItemParseResult<Price> {
    // note: input must not be empty
    match PRICE_PATT.captures(input) {
        Some(patts) => {
            let (_, [value_s, unit]) = patts.extract();
            let value = value_s
                .split_whitespace()
                .collect::<String>()
                .parse::<i32>()
                .map_err(|_| ItemParseError::InvalidPrice(input.to_string()))?;
            Ok(Price {
                value: value,
                unit: unit.to_string(),
            })
        }
        None => Err(ItemParseError::InvalidPrice(input.to_string())),
    }
}

impl Parser {
    pub fn new(user_tz: Tz, server_time: DateTime<Utc>) -> Self {
        let user_today = server_time.with_timezone(&user_tz);
        let user_yesterday = user_today.sub(Days::new(1));

        Parser {
            server_time: server_time,
            user_today: user_today,
            user_yesterday: user_yesterday,
        }
    }

    fn parse_rel_time(&self, relday_s: &str, hhmm_s: &str) -> ParseResult<DateTime<Utc>> {
        let hhmm = parse_hh_mm(hhmm_s)?;

        let date = match relday_s {
            "tänään" => Ok(self.user_today.clone()),
            "eilen" => Ok(self.user_yesterday.clone()),
            _ => Err(ParseError::InvalidRelativeDay(relday_s.to_string())),
        }?;

        let new_ts_maybe = date.timezone().with_ymd_and_hms(
            date.year(),
            date.month(),
            date.day(),
            hhmm.hour(),
            hhmm.minute(),
            0,
        );

        match new_ts_maybe {
            LocalResult::Single(new_ts) => Ok(new_ts.with_timezone(&Utc)),
            _ => Err(ParseError::ArithmeticProblem),
        }
    }

    fn parse_abs_time(
        &self,
        day_s: &str,
        month_s: &str,
        hhmm_s: &str,
    ) -> ParseResult<DateTime<Utc>> {
        let day = parse_day(day_s)?;
        let month = parse_month_short(month_s)?;
        let hhmm = parse_hh_mm(hhmm_s)?;

        let new_ts_maybe = self.user_today.timezone().with_ymd_and_hms(
            self.user_today.year(),
            month.number_from_month(),
            day,
            hhmm.hour(),
            hhmm.minute(),
            0,
        );

        let new_ts = match new_ts_maybe {
            LocalResult::Single(new_ts) => Ok(new_ts),
            _ => Err(ParseError::ArithmeticProblem),
        }?;

        // timestamp can be in the future; check manually since we lack the actual year.
        // this assumes no item can be listed for over a year.
        let y_offset = if new_ts > self.server_time { 1 } else { 0 };

        let new_ts = new_ts
            .with_year(new_ts.year() - y_offset)
            .ok_or(ParseError::ArithmeticProblem)?;

        Ok(new_ts.with_timezone(&Utc))
    }

    pub fn parse_posted_at(&self, ts: &str) -> ParseResult<DateTime<Utc>> {
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

    pub fn parse_document(&self, doc: &Html) -> Result<Vec<Item>, ItemParseError> {
        let mut items = vec![];

        for element in doc.select(&ROW_SELECTOR) {
            let id = element
                .attr("id")
                .ok_or(ItemParseError::MissingTitle)
                .map(|s| remove_prefix_maybe("item_", &s))?;

            let company_ad = element
                .attr("data-company-ad")
                .ok_or(ItemParseError::MissingCompanyAd)
                .and_then(|s| s.parse::<u8>().map_err(|_| ItemParseError::UnexpectedValue))?
                != 0;

            let href = element.attr("href").ok_or(ItemParseError::MissingHref)?;

            let price = match {
                element
                    .select(&PRICE_SELECTOR)
                    .next()
                    .map(|n| n.text().collect::<String>())
                    .filter(|s| !s.is_empty())
            } {
                None => None,
                Some(t) => Some(price_parse(&t)?),
            };

            let img = element
                .select(&IMAGE_SELECTOR)
                .next()
                .ok_or(ItemParseError::MissingImg)?
                .attr("src")
                .ok_or(ItemParseError::MissingImg)?;

            let title = element
                .select(&TITLE_SELECTOR)
                .next()
                .ok_or(ItemParseError::MissingTitle)
                .map(|s| s.inner_html())?;

            let posted_at = element
                .select(&POSTED_AT_SELECTOR)
                .next()
                .ok_or(ItemParseError::MissingPostedAt)
                .map(|s| reformat_ws(&s.inner_html()))?;

            let posted_at_parsed = self.parse_posted_at(&posted_at).expect("fukken ded");

            let mut combined = element.select(&COMBINED_SELECTOR);

            let location = combined
                .next()
                .map(|n| reformat_ws(&n.inner_html()))
                .ok_or(ItemParseError::MissingLocation)?;

            let direction = combined
                .next()
                .map(|n| reformat_ws(&n.inner_html()))
                .ok_or(ItemParseError::MissingDirection)?;

            let seller_maybe = match {
                combined
                    .map(|n| reformat_ws(&n.inner_html()))
                    .collect::<Vec<String>>()
            } {
                v if v.len() == 0 => None,
                v => Some(v.join(" ")),
            };

            let item = Item {
                id: id.to_string(),
                company_ad: company_ad,
                href: href.to_string(),
                price: price,
                img: img.to_string(),
                title: title.trim().to_string(),
                posted_at_orig: posted_at.to_string(),
                posted_at: posted_at_parsed,
                location: location.to_string(),
                direction: direction.to_string(),
                seller: seller_maybe,
            };

            items.push(item);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let parent = Path::new(file!()).parent().unwrap();
        let path = &parent.join("testdata/dump.html");
        parse_file(&path);
    }

    #[test]
    fn parse_ws() {
        let s = "\t\t\t\t\tfoo\t\t\tbar\t\t".to_string();
        let y = reformat_ws(&s);
        assert_eq!("foo bar".to_string(), y);
    }

    #[test]
    fn parse_price() {
        assert_eq!(
            price_parse("1 €"),
            Ok(Price {
                value: 1,
                unit: "€".to_string(),
            })
        );
        assert_eq!(
            price_parse(" 1 599  €"),
            Ok(Price {
                value: 1599,
                unit: "€".to_string(),
            })
        );
    }

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
        let parser = Parser::new(chrono_tz::Europe::Helsinki, get_time());
        let result = parser.parse_posted_at("tänään 01:23");
        assert_eq!(
            result,
            Ok(chrono_tz::Europe::Helsinki
                .with_ymd_and_hms(2023, 3, 25, 1, 23, 0)
                .unwrap()
                .with_timezone(&Utc))
        );

        let result = parser.parse_posted_at("eilen 15:59");
        assert_eq!(
            result,
            Ok(chrono_tz::Europe::Helsinki
                .with_ymd_and_hms(2023, 3, 24, 15, 59, 0)
                .unwrap()
                .with_timezone(&Utc))
        );

        let result = parser.parse_posted_at("tänään 25:48");
        assert_eq!(result, Err(ParseError::InvalidTime("25:48".to_string())));
    }

    #[test]
    fn test_parse_ts_absolute() {
        let parser = Parser::new(chrono_tz::Europe::Helsinki, get_time());
        let result = parser.parse_posted_at("21 huh 19:52");
        assert_eq!(
            result,
            Ok(chrono_tz::Europe::Helsinki
                .with_ymd_and_hms(2022, 4, 21, 19, 52, 0)
                .unwrap()
                .with_timezone(&Utc))
        );
        let result = parser.parse_posted_at("32 tam 01:32");
        assert_eq!(result, Err(ParseError::InvalidDay("32".to_string())));
    }

    #[test]
    fn tz_temppu() {
        use chrono_tz::Europe;

        match Europe::Helsinki.with_ymd_and_hms(2024, 01, 29, 19, 03, 0) {
            LocalResult::Single(tss) => {
                println!("tss: {:?}", tss);
                println!("utc: {:?}", tss.with_timezone(&Utc));
            }
            _ => panic!("omg"),
        }
    }

    #[test]
    fn test_parse_day() {
        assert!(parse_day("0").is_err());
        assert!(parse_day("32").is_err());
        assert!(parse_day("1").unwrap() == 1);
        assert!(parse_day("31").unwrap() == 31);
    }
}

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
