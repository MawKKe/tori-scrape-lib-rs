pub fn add(left: usize, right: usize) -> usize {
    left + right
}

use chrono::Month;

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
}
