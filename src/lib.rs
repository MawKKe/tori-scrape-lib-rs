use std::fs;
use std::path::Path;

use chrono::NaiveDateTime;
use chrono::{DateTime, Datelike, Days, LocalResult, Month, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use encoding_rs;
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
    pub item_id: String,
    pub direction: String,
    pub title: String,
    pub price: Option<Price>,
    pub location: String,
    pub seller: Option<String>,
    pub is_company_ad: bool,
    pub href: String,
    pub thumbnail_url: Option<String>,
    pub posted_at_orig: String,
    pub posted_at: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub enum ItemParseErrorKind {
    MissingID,
    MissingTitle,
    MissingHref,
    MissingCompanyAd,
    MissingImg,
    MissingPostedAt,
    MissingLocation,
    MissingDirection,
    UnexpectedValue(&'static str, String),
    InvalidPrice(String),
    InvalidDate(DateParseError),
}

#[derive(Debug, PartialEq)]
pub struct ItemParseError {
    pub item_idx: usize,
    pub item_id: Option<String>,
    pub error: ItemParseErrorKind,
}

#[derive(Debug, PartialEq)]
pub enum DateParseError {
    InvalidHighlevelStructure(String),
    InvalidDay(String),
    InvalidTime(String),
    InvalidMonth(String),
    InvalidRelativeDay(String),
    ArithmeticProblem,
}

pub type DateParseResult<T> = Result<T, DateParseError>;

type ItemParseResult<T> = Result<T, ItemParseError>;

pub struct Parser {
    user_today: DateTime<chrono_tz::Tz>,
    user_yesterday: DateTime<chrono_tz::Tz>,
}

pub fn encoding_lookup(name: &str) -> Option<&'static encoding_rs::Encoding> {
    match name {
        "ISO_8859_15" => Some(encoding_rs::ISO_8859_15),
        _ => Some(encoding_rs::UTF_8),
    }
}

pub fn timezone_lookup(name: &str) -> Result<Tz, String> {
    name.parse::<Tz>()
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

fn price_parse(input: &str) -> Result<Price, ItemParseErrorKind> {
    // note: input must not be empty
    match PRICE_PATT.captures(input) {
        Some(patts) => {
            let (_, [value_s, unit]) = patts.extract();
            let value = value_s
                .split_whitespace()
                .collect::<String>()
                .parse::<i32>()
                .map_err(|_| ItemParseErrorKind::InvalidPrice(input.to_string()))?;
            Ok(Price {
                value: value,
                unit: unit.to_string(),
            })
        }
        None => Err(ItemParseErrorKind::InvalidPrice(input.to_string())),
    }
}

fn parse_month_short(month_short_name: &str) -> DateParseResult<Month> {
    match &month_short_name.to_lowercase()[..] {
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
        _ => Err(DateParseError::InvalidMonth(month_short_name.to_string())),
    }
}

fn parse_hh_mm(time: &str) -> DateParseResult<NaiveTime> {
    NaiveTime::parse_from_str(time, "%H:%M")
        .map_err(|_| DateParseError::InvalidTime(time.to_string()))
}

fn parse_day(day: &str) -> DateParseResult<u32> {
    match day.parse::<u32>() {
        Ok(d) if d >= 1 && d <= 31 => Ok(d),
        _ => Err(DateParseError::InvalidDay(day.to_string())),
    }
}

impl Parser {
    pub fn new(fetch_time: DateTime<Tz>) -> Self {
        Parser {
            user_today: fetch_time,
            user_yesterday: fetch_time.sub(Days::new(1)),
        }
    }

    fn parse_rel_time(&self, relday_s: &str, hhmm_s: &str) -> DateParseResult<DateTime<Utc>> {
        let naive_time = parse_hh_mm(hhmm_s)?;

        let naive_date = match relday_s {
            "tänään" => Ok(self.user_today.date_naive()),
            "eilen" => Ok(self.user_yesterday.date_naive()),
            _ => Err(DateParseError::InvalidRelativeDay(relday_s.to_string())),
        }?;

        let date = NaiveDateTime::new(naive_date, naive_time);

        match self.user_today.timezone().from_local_datetime(&date) {
            LocalResult::Single(new_ts) => Ok(new_ts.with_timezone(&Utc)),
            _ => Err(DateParseError::ArithmeticProblem),
        }
    }

    fn parse_abs_time(
        &self,
        day_s: &str,
        month_s: &str,
        hhmm_s: &str,
    ) -> DateParseResult<DateTime<Utc>> {
        let day = parse_day(day_s)?;
        let month = parse_month_short(month_s)?;
        let naive_time = parse_hh_mm(hhmm_s)?;
        let new_ts_maybe = self.user_today.timezone().with_ymd_and_hms(
            self.user_today.year(),
            month.number_from_month(),
            day,
            naive_time.hour(),
            naive_time.minute(),
            0,
        );

        let new_ts = match new_ts_maybe {
            LocalResult::Single(new_ts) => Ok(new_ts),
            _ => Err(DateParseError::ArithmeticProblem),
        }?;

        // timestamp can be in the future; check manually since we lack the actual year.
        // this assumes no item can be listed for over a year.
        let y_offset = if new_ts > self.user_today { 1 } else { 0 };

        let new_ts = new_ts
            .with_year(new_ts.year() - y_offset)
            .ok_or(DateParseError::ArithmeticProblem)?;

        Ok(new_ts.with_timezone(&Utc))
    }

    pub fn parse_posted_at(&self, ts: &str) -> DateParseResult<DateTime<Utc>> {
        if let Some(patts) = REL_TIME.captures(ts) {
            let (_, [relday_s, hhmm_s]) = patts.extract();
            self.parse_rel_time(relday_s, hhmm_s)
        } else if let Some(patts) = ABS_TIME.captures(ts) {
            let (_, [day_s, month_s, hhmm_s]) = patts.extract();
            self.parse_abs_time(day_s, month_s, hhmm_s)
        } else {
            Err(DateParseError::InvalidHighlevelStructure(ts.to_string()))
        }
    }

    pub fn parse_document(&self, doc: &Html) -> ItemParseResult<Vec<Item>> {
        let mut items = vec![];
        use ItemParseErrorKind::*;

        for (i, element) in doc.select(&ROW_SELECTOR).enumerate() {
            let item_id = {
                let item_id = element.attr("id").ok_or(ItemParseError {
                    item_idx: i,
                    item_id: None,
                    error: MissingID,
                })?;

                item_id
                    .strip_prefix("item_")
                    .ok_or(ItemParseError {
                        item_idx: i,
                        item_id: None,
                        error: UnexpectedValue("id", item_id.to_string()),
                    })?
                    .to_string()
            };
            let is_company_ad = {
                let s = element.attr("data-company-ad").ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingCompanyAd,
                })?;

                match s {
                    "0" => Ok(false),
                    "1" => Ok(true),
                    _ => Err(ItemParseError {
                        item_idx: i,
                        item_id: Some(item_id.clone()),
                        error: UnexpectedValue("data-company-ad", s.to_string()),
                    }),
                }
            }?;

            let href = element
                .attr("href")
                .map(|s| s.to_string())
                .ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingHref,
                })?;

            let price = match {
                element
                    .select(&PRICE_SELECTOR)
                    .next()
                    .map(|n| n.text().collect::<String>())
                    .filter(|s| !s.is_empty())
            } {
                None => None,
                Some(t) => Some(price_parse(&t).unwrap()), // FIXME
            };

            let thumbnail_url = element
                .select(&IMAGE_SELECTOR)
                .next()
                .and_then(|n| n.attr("src"))
                .map(|s| s.to_string());

            let title = element
                .select(&TITLE_SELECTOR)
                .next()
                .map(|s| s.inner_html())
                .ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingTitle,
                })?
                .trim()
                .to_string();

            let posted_at = element
                .select(&POSTED_AT_SELECTOR)
                .next()
                .map(|s| reformat_ws(&s.inner_html()))
                .ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingPostedAt,
                })?;

            let posted_at_parsed =
                self.parse_posted_at(&posted_at)
                    .map_err(|e| ItemParseError {
                        item_idx: i,
                        item_id: Some(item_id.clone()),
                        error: InvalidDate(e),
                    })?;

            let mut combined = element.select(&COMBINED_SELECTOR);

            let location = combined
                .next()
                .map(|n| reformat_ws(&n.inner_html()))
                .ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingLocation,
                })?;

            let direction = combined
                .next()
                .map(|n| reformat_ws(&n.inner_html()))
                .ok_or(ItemParseError {
                    item_idx: i,
                    item_id: Some(item_id.clone()),
                    error: MissingDirection,
                })?;

            let seller_maybe = {
                let v: Vec<String> = combined.map(|n| reformat_ws(&n.inner_html())).collect();
                match v.len() {
                    0 => None,
                    _ => Some(v.join(" ")),
                }
            };

            let item = Item {
                item_id: item_id,
                direction: direction,
                title: title,
                is_company_ad: is_company_ad,
                href: href,
                price: price,
                thumbnail_url: thumbnail_url,
                posted_at_orig: posted_at,
                posted_at: posted_at_parsed,
                location: location,
                seller: seller_maybe,
            };

            items.push(item);
        }
        Ok(items)
    }

    pub fn parse_from_string(&self, buf: &str) -> ItemParseResult<Vec<Item>> {
        let doc = Html::parse_document(buf);
        self.parse_document(&doc)
    }
}

pub fn decode_to_string(path: &Path, encoding: &'static encoding_rs::Encoding) -> String {
    let file = fs::File::open(path).unwrap();

    let transcoded = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(file);

    let mut reader = BufReader::new(transcoded);

    let mut buf = String::new();

    let n = reader.read_to_string(&mut buf).unwrap();

    assert!(n > 0);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file() {
        let tz = "Europe/Helsinki".parse::<Tz>().unwrap();
        let parent = Path::new(file!()).parent().unwrap();

        let test_data = vec![
            (
                "testdata/2023-03-25-105201-dump.html",
                tz.with_ymd_and_hms(2023, 03, 25, 10, 52, 01).unwrap(),
                40,
            ),
            (
                "testdata/2024-01-30-123020-dump.html",
                tz.with_ymd_and_hms(2024, 1, 30, 12, 30, 20).unwrap(),
                12,
            ),
        ];

        for (path, fetch_time, expect_num_items) in test_data {
            let path = &parent.join(path);
            let buf = decode_to_string(path, encoding_lookup("ISO_8859_15").unwrap());
            let parser = Parser::new(fetch_time);
            let result = parser.parse_from_string(&buf).unwrap();
            assert_eq!(result.len(), expect_num_items);
        }
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
            Err(DateParseError::InvalidMonth("foo".to_string()))
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
            Err(DateParseError::InvalidTime("01:60".to_string()))
        );
        assert_eq!(
            parse_hh_mm("25:24"),
            Err(DateParseError::InvalidTime("25:24".to_string()))
        );
    }

    fn get_time() -> DateTime<Tz> {
        timezone_lookup("Europe/Helsinki")
            .unwrap()
            .with_ymd_and_hms(2023, 3, 25, 10, 52, 1)
            .unwrap()
    }

    #[test]
    fn test_parse_ts_relative() {
        let parser = Parser::new(get_time());

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
        assert_eq!(
            result,
            Err(DateParseError::InvalidTime("25:48".to_string()))
        );
    }

    #[test]
    fn test_parse_ts_absolute() {
        let parser = Parser::new(get_time());
        let result = parser.parse_posted_at("21 huh 19:52");
        assert_eq!(
            result,
            Ok(chrono_tz::Europe::Helsinki
                .with_ymd_and_hms(2022, 4, 21, 19, 52, 0)
                .unwrap()
                .with_timezone(&Utc))
        );
        let result = parser.parse_posted_at("32 tam 01:32");
        assert_eq!(result, Err(DateParseError::InvalidDay("32".to_string())));
    }

    #[test]
    fn test_parse_day() {
        assert!(parse_day("0").is_err());
        assert!(parse_day("32").is_err());
        assert!(parse_day("1").unwrap() == 1);
        assert!(parse_day("31").unwrap() == 31);
    }

    #[test]
    fn test_read_json() {
        use serde_json;

        let parent = Path::new(file!()).parent().unwrap();
        let path = &parent.join("testdata/test.json");
        let file = fs::File::open(path).unwrap();
        let json: serde_json::Value = serde_json::from_reader(file).unwrap();
        assert_eq!(json["name"], "John Doe");
    }
}

pub fn reformat_ws(input: &str) -> String {
    let w = input.split_whitespace();
    w.collect::<Vec<&str>>().join(" ")
}

pub fn remove_prefix_maybe(prefix: &str, input: &str) -> String {
    input.strip_prefix(prefix).unwrap_or(input).to_string()
}

#[cfg(test)]
mod utils_tests {
    use super::*;

    #[test]
    fn test_remove_prefix() {
        assert_eq!(remove_prefix_maybe("testi_", "testi_data"), "data");
        assert_eq!(remove_prefix_maybe("notexist_", "testi_data"), "testi_data");
    }

    #[test]
    fn test_reformat_ws() {
        let s = "\t\t\t\t\tfoo\t\t\tbar\t\t".to_string();
        let y = reformat_ws(&s);
        assert_eq!("foo bar".to_string(), y);
    }
}
