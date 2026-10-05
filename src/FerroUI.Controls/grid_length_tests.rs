use crate::{GridLength, GridUnitType};

#[test]
fn parse_should_parse_auto() {
    let result = GridLength::parse("Auto").unwrap();

    assert_eq!(GridLength::AUTO, result);
}

#[test]
fn parse_should_parse_auto_lowercase() {
    let result = GridLength::parse("auto").unwrap();

    assert_eq!(GridLength::AUTO, result);
}

#[test]
fn parse_should_parse_star() {
    let result = GridLength::parse("*").unwrap();

    assert_eq!(GridLength::new(1.0, GridUnitType::Star), result);
}

#[test]
fn parse_should_parse_star_value() {
    let result = GridLength::parse("2*").unwrap();

    assert_eq!(GridLength::new(2.0, GridUnitType::Star), result);
}

#[test]
fn parse_should_parse_pixel_value() {
    let result = GridLength::parse("2").unwrap();

    assert_eq!(GridLength::new(2.0, GridUnitType::Pixel), result);
}

#[test]
fn parse_should_fail_with_format_error_for_invalid_string() {
    assert!(GridLength::parse("2x").is_err());
    assert!("2x".parse::<GridLength>().is_err());
}

#[test]
fn parse_lengths_accepts_comma_separators() {
    let result = GridLength::parse_lengths("*,Auto,2*,4").unwrap();

    assert_eq!(
        vec![
            GridLength::new(1.0, GridUnitType::Star),
            GridLength::AUTO,
            GridLength::new(2.0, GridUnitType::Star),
            GridLength::new(4.0, GridUnitType::Pixel),
        ],
        result
    );
}

#[test]
fn parse_lengths_accepts_space_separators() {
    let result = GridLength::parse_lengths("* Auto 2* 4").unwrap();

    assert_eq!(
        vec![
            GridLength::new(1.0, GridUnitType::Star),
            GridLength::AUTO,
            GridLength::new(2.0, GridUnitType::Star),
            GridLength::new(4.0, GridUnitType::Pixel),
        ],
        result
    );
}

#[test]
fn parse_lengths_accepts_comma_separators_with_spaces() {
    let result = GridLength::parse_lengths("*, Auto, 2* ,4").unwrap();

    assert_eq!(
        vec![
            GridLength::new(1.0, GridUnitType::Star),
            GridLength::AUTO,
            GridLength::new(2.0, GridUnitType::Star),
            GridLength::new(4.0, GridUnitType::Pixel),
        ],
        result
    );
}

// The reference test formats under every culture; formatting here is always
// culture invariant.
#[test]
fn to_string_should_pass() {
    for (d, type_, result) in [
        (1.2, GridUnitType::Pixel, "1.2"),
        (1.2, GridUnitType::Star, "1.2*"),
        (1.2, GridUnitType::Auto, "Auto"),
    ] {
        let length = GridLength::new(d, type_);
        assert_eq!(result, length.to_string());
    }
}

// The tests below are not part of the reference suite.

#[test]
fn default_is_auto_with_zero_value() {
    let length = GridLength::default();

    assert!(length.is_auto());
    assert_eq!(0.0, length.value());
    assert_eq!(GridLength::AUTO, length);
}

#[test]
fn auto_lengths_are_equal_whatever_their_value() {
    assert_eq!(GridLength::new(20.0, GridUnitType::Auto), GridLength::AUTO);
    assert_ne!(
        GridLength::new(1.0, GridUnitType::Star),
        GridLength::new(1.0, GridUnitType::Pixel)
    );
}

#[test]
fn parse_rejects_values_that_are_not_valid_lengths() {
    assert!(GridLength::parse("-1").is_err());
    assert!(GridLength::parse("-1*").is_err());
    assert!(GridLength::parse("NaN").is_err());
    assert!(GridLength::parse("Infinity").is_err());
    assert!(GridLength::parse("").is_err());
}

#[test]
#[should_panic(expected = "Invalid value")]
fn new_panics_for_negative_value() {
    GridLength::new(-1.0, GridUnitType::Pixel);
}
