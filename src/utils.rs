use std::fs;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;

use chrono_tz::Tz;
use encoding_rs;
use encoding_rs_io::DecodeReaderBytesBuilder;

/// Reads given file (assumed to be in given encoding), and transcodes it to native UTF-8 String.
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

/// Lookup locale encoding using conventional string labels such as "ISO_8859_15"
pub fn encoding_lookup(name: &str) -> Option<&'static encoding_rs::Encoding> {
    match name {
        "ISO_8859_15" => Some(encoding_rs::ISO_8859_15),
        _ => Some(encoding_rs::UTF_8),
    }
}

/// Lookup timezone using conventional string labels such as "Europe/Helsinnki"
pub fn timezone_lookup(name: &str) -> Result<Tz, String> {
    name.parse::<Tz>()
}

/// Takes a string with uncontrolled amount of whitespace between tokens,
/// and returns the string reformatted with single space characters between
/// tokens.
///
/// # Examples
///
/// ```
/// use tori_scrape::utils::reformat_ws;
///
/// assert_eq!(reformat_ws("   foo      bar baz  "), "foo bar baz".to_string());
/// ```
pub fn reformat_ws(input: &str) -> String {
    let w = input.split_whitespace();
    w.collect::<Vec<&str>>().join(" ")
}
