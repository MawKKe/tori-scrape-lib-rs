pub mod dateparser;

use scraper::Html;

//pub fn parse(content: &str, dp: &dateparser::DateParser) {}

use std::fs;
use std::path::{Path, PathBuf};

use encoding_rs::ISO_8859_15;
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::io::BufReader;
use std::io::Read;

use scraper::Selector;

pub fn parse_file(path: &Path) {
    let file = fs::File::open(path).unwrap();
    let transcoded = DecodeReaderBytesBuilder::new()
        .encoding(Some(ISO_8859_15))
        .build(file);

    let mut reader = BufReader::new(transcoded);
    let mut buf = String::new();
    let n = reader.read_to_string(&mut buf).unwrap();
    assert!(buf.len() > 0);
    let doc = Html::parse_document(&buf);

    let selector = Selector::parse("a[data-row]").unwrap();
    let title_s = Selector::parse("div .li-title").unwrap();

    for element in doc.select(&selector) {
        let id = element.attr("id").unwrap_or("null");
        let id = id.strip_prefix("item_").unwrap_or(id);
        let company_ad = element.attr("data-company-ad").unwrap_or("0") == "1";
        let href = element.attr("href").unwrap_or("null");
        let title = element.select(&title_s).next().unwrap();
        println!(
            "elem: id={}, title='{}', ad={}, href={}",
            id,
            title.text().collect::<String>().trim(),
            company_ad,
            href
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment() {
        let html = r#"
    <!DOCTYPE html>
    <meta charset="utf-8">
    <title>Hello, world!</title>
    <h1 class="foo">Hello, <i>world!</i></h1>
"#;

        let document = Html::parse_document(html);
    }

    #[test]
    fn read() {
        let parent = Path::new(file!()).parent().unwrap();
        let path = &parent.join("testdata/dump.html");
        parse_file(&path);
    }
}
