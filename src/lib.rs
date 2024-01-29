pub mod dateparser;

use chrono::{DateTime, Utc};
use scraper::Html;

use std::fs;
use std::path::Path;

use encoding_rs::ISO_8859_15;
use encoding_rs_io::DecodeReaderBytesBuilder;
use lazy_static::lazy_static;
use regex::Regex;
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
    pub posted_at: String,
    pub posted_at_parsed: DateTime<Utc>,
    pub location: String,
    pub direction: String,
    pub seller: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ItemParseError {
    Success,
    MissingID,
    MissingTitle,
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
    let mut items: Vec<Item> = vec![];
    let result = parser.parse_document(&doc, &mut items);
    match result {
        Ok(_) => {
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
    static ref IMAGE_SELECTOR: Selector = Selector::parse("div .item_image").unwrap();
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

use chrono_tz::Tz;

impl Parser {
    pub fn new(user_tz: Tz, server_time: DateTime<Utc>) -> Self {
        Parser {
            user_tz,
            server_time,
        }
    }
    pub fn parse_document(&self, doc: &Html, items: &mut Vec<Item>) -> Result<(), ItemParseError> {
        let dp = dateparser::DateParser::new(self.server_time, self.user_tz);
        for element in doc.select(&ROW_SELECTOR) {
            let id = element.attr("id").unwrap_or("null");

            let id = id.strip_prefix("item_").unwrap_or(id);

            let company_ad = element.attr("data-company-ad").unwrap_or("0") == "1";

            let href = element.attr("href").unwrap_or("null");

            let price_maybe = element.select(&PRICE_SELECTOR).next();
            let price_maybe = price_maybe.map(|n| n.text().collect::<String>());
            let price_maybe = price_maybe.filter(|s| !s.is_empty());

            let price = match price_maybe {
                None => None,
                Some(t) => Some(price_parse(&t)?),
            };

            let img = element
                .select(&IMAGE_SELECTOR)
                .next()
                .unwrap()
                .attr("src")
                .unwrap_or("");

            let title_node = element.select(&TITLE_SELECTOR).next().unwrap();
            let title = title_node.text().next().unwrap();

            let posted_at_node = element.select(&POSTED_AT_SELECTOR).next().unwrap();
            let posted_at = posted_at_node.text().next().map(reformat_ws).unwrap();

            let posted_at_parsed = dp.parse(&posted_at).expect("fukken ded");

            let combined = element
                .select(&COMBINED_SELECTOR)
                .map(|n| reformat_ws(&n.inner_html()))
                .collect::<Vec<String>>();

            let location = &combined[0];

            let direction = &combined[1];

            let seller_maybe = if combined.len() > 2 {
                Some(combined[2..].join(" "))
            } else {
                None
            };

            let item = Item {
                id: id.to_string(),
                company_ad: company_ad,
                href: href.to_string(),
                price: price,
                img: img.to_string(),
                title: title.trim().to_string(),
                posted_at: posted_at.to_string(),
                posted_at_parsed: posted_at_parsed,
                location: location.to_string(),
                direction: direction.to_string(),
                seller: seller_maybe,
            };
            items.push(item);
        }
        Ok(())
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
