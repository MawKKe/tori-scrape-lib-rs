pub fn add(left: usize, right: usize) -> usize {
    left + right
}

use chrono::{Month, NaiveDate, NaiveTime, ParseResult, Utc};
use regex::Regex;

fn parse_month_short(short_name: &str) -> Result<Month, String> {
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
        _ => Err(format!("unknown month: '{short_name}'")),
    }
}
fn parse_hh_mm(time: &str) -> ParseResult<NaiveTime> {
    NaiveTime::parse_from_str(time, "%H:%M")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_parse_month_short() {
        assert_eq!(parse_month_short("tam").expect("wtf"), Month::January);
    }
    #[test]
    fn test_parse_hh_mm() {
        let hhmm = parse_hh_mm("01:23").expect("huh");
        assert_eq!(hhmm, NaiveTime::from_hms_opt(1, 23, 0).unwrap());
        assert!(parse_hh_mm("01:60").is_err());
    }

    #[test]
    fn test_parse_today() {
        // server_time := time.Date(2023, 3, 25, 10, 52, 1, 0, time.UTC)

        let server_time = NaiveDate::from_ymd_opt(2023, 3, 25)
            .unwrap()
            .and_hms_nano_opt(10, 52, 1, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        //let words = "21 hel 19:59".to_string().split_whitespace().into();

        // HUOM: tori.fi päivämäärät ovat aina suhteellisia *nyt* hetkeen nähden
        let testdata = vec!["tänään 01:23", "eilen 15:44", "21 hel 19:59"];
    }

    #[test]
    fn test_regex_split_relative() {
        let s = "tänään 01:23";
        let re = Regex::new(r"\s*(\w+)\s+(\d{2}:\d{2})").unwrap();
        let (_, [relday, hhmm]) = re.captures(s).unwrap().extract();
        let result = (relday, parse_hh_mm(hhmm).unwrap());
        assert_eq!(
            result,
            ("tänään", NaiveTime::from_hms_opt(1, 23, 0).unwrap())
        )
    }

    #[test]
    fn test_regex_split_explicit() {
        let s = "21 hel 19:59";
        let re = Regex::new(r"\s*(\d{1,2})\s+([a-zA-Z]{3})\s+(\d{2}:\d{2})\s*").unwrap();

        let (_, [day, month, hhmm]) = re.captures(s).unwrap().extract();
        let result = (
            day.parse::<u8>().unwrap(),
            parse_month_short(month).unwrap(),
            parse_hh_mm(hhmm).unwrap(),
        );

        assert_eq!(
            result,
            (
                21,
                Month::February,
                NaiveTime::from_hms_micro_opt(19, 59, 0, 0).unwrap()
            )
        )
    }
}
