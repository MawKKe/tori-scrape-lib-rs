pub mod dateparser;

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use encoding_rs::ISO_8859_15;
use encoding_rs_io::DecodeReaderBytesBuilder;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::Html;
use scraper::Selector;
use std::io::BufReader;
use std::io::Read;

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

type ItemParseResult<T> = Result<T, ItemParseError>;

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

struct Parser {
    user_tz: Tz,
    server_time: DateTime<Utc>,
}

impl Parser {
    pub fn new(user_tz: Tz, server_time: DateTime<Utc>) -> Self {
        Parser {
            user_tz,
            server_time,
        }
    }
    pub fn parse_document(&self, doc: &Html) -> Result<Vec<Item>, ItemParseError> {
        let mut items = vec![];
        let dp = dateparser::DateParser::new(self.server_time, self.user_tz);
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

            let posted_at_parsed = dp.parse(&posted_at).expect("fukken ded");

            /*
            let combined = element
                .select(&COMBINED_SELECTOR)
                .map(|n| reformat_ws(&n.inner_html()))
                .collect::<Vec<String>>();

            if combined.len() < 2 {
                return Err(ItemParseError::NotEnoughItems);
            }

            let [location, direction] = &combined[0..1];

            let seller_maybe = if combined.len() > 2 {
                Some(combined[2..].join(" "))
            } else {
                None
            };
            */
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
}
