use std::env;
use std::path::Path;
use tori_scrape::parse_file;

fn main() {
    let args: Vec<String> = env::args().collect();
    parse_file(&Path::new(&args[1]));
}
