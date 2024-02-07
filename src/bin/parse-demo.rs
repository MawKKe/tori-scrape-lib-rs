use chrono::Utc;
use std::env;
use std::path::Path;
use tori_scrape::utils;
use tori_scrape::Parser;

use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    let tz = utils::timezone_lookup("Europe/Helsinki").unwrap();

    let fetch_time = Utc::now().with_timezone(&tz);

    let parser = Parser::new(fetch_time);

    let buf = utils::decode_to_string(
        Path::new(&args[1]),
        utils::encoding_lookup("ISO_8859_15").unwrap(),
    );

    let start = Instant::now();
    let result = parser.parse_from_string(&buf);
    let duration = start.elapsed();

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

    println!("took: {:?}", duration);
}
