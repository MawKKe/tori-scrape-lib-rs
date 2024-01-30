use std::env;
use std::path::Path;
use tori_scrape::{decode_to_string, encoding_lookup, timezone_lookup, Parser};

use chrono::Utc;

use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    let tz = timezone_lookup("Europe/Helsinki").unwrap();

    let fetch_time = Utc::now().with_timezone(&tz);

    let parser = Parser::new(fetch_time);

    let buf = decode_to_string(Path::new(&args[1]), encoding_lookup("ISO_8859_15").unwrap());

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
