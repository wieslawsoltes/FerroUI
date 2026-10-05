use super::{FlexBasis, FlexBasisKind};

#[test]
fn parse_should_parse_auto() {
    let result = FlexBasis::parse("Auto").unwrap();

    assert_eq!(result, FlexBasis::AUTO);
}

#[test]
fn parse_should_parse_auto_lowercase() {
    let result = FlexBasis::parse("auto").unwrap();

    assert_eq!(result, FlexBasis::AUTO);
}

#[test]
fn parse_should_parse_percentage() {
    let result = FlexBasis::parse("50%").unwrap();

    assert_eq!(result, FlexBasis::new(0.5, FlexBasisKind::Relative));
}

#[test]
fn parse_should_parse_absolute_value() {
    let result: FlexBasis = "2".parse().unwrap();

    assert_eq!(result, FlexBasis::new(2.0, FlexBasisKind::Absolute));
}

#[test]
fn parse_should_throw_argument_exception_for_invalid_string() {
    assert!(FlexBasis::parse("2x").is_err());
}

/// Upstream runs this under two current cultures; the text form here never
/// reads a current culture, so the assertion is made once.
#[test]
fn to_string_all_culture_absolute_should_pass() {
    let length = FlexBasis::new(1.2, FlexBasisKind::Absolute);

    assert_eq!(length.to_string(), "1.2");
}

// Additional tests (not ports): the text forms checked against the reference
// runtime.

#[test]
fn additional_to_string_matches_the_general_17_digit_format() {
    for (value, kind, expected) in TO_STRING_ROWS {
        assert_eq!(FlexBasis::new(*value, *kind).to_string(), *expected, "{value} {kind:?}");
    }
}

#[test]
fn additional_parse_edge_cases() {
    for (text, expected) in PARSE_ROWS {
        assert_eq!(FlexBasis::parse(text).ok(), *expected, "{text:?}");
    }
}

/// Equality as upstream (`Equals`/`==`): two auto bases are equal whatever
/// their values, everything else compares value and kind. The hash agrees
/// with it (upstream hashes the value of an auto basis too, so two equal auto
/// bases with different values hash differently there).
#[test]
fn additional_equality_and_hash_agree() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(basis: FlexBasis) -> u64 {
        let mut hasher = DefaultHasher::new();
        basis.hash(&mut hasher);
        hasher.finish()
    }

    let auto_with_value = FlexBasis::from_parts(5.0, FlexBasisKind::Auto);
    assert_eq!(FlexBasis::AUTO, auto_with_value);
    assert_eq!(hash_of(FlexBasis::AUTO), hash_of(auto_with_value));
    assert_eq!(FlexBasis::default(), FlexBasis::AUTO);
    assert_eq!(FlexBasis::auto(), FlexBasis::AUTO);

    // Positive and negative zero are equal values.
    let negative_zero = FlexBasis::from_parts(-0.0, FlexBasisKind::Absolute);
    assert_eq!(FlexBasis::absolute(0.0), negative_zero);
    assert_eq!(hash_of(FlexBasis::absolute(0.0)), hash_of(negative_zero));

    assert_eq!(FlexBasis::absolute(2.0), FlexBasis::new(2.0, FlexBasisKind::Absolute));
    assert_eq!(hash_of(FlexBasis::absolute(2.0)), hash_of(FlexBasis::new(2.0, FlexBasisKind::Absolute)));
    assert_ne!(FlexBasis::absolute(2.0), FlexBasis::absolute(3.0));
    assert_ne!(FlexBasis::absolute(2.0), FlexBasis::new(2.0, FlexBasisKind::Relative));
    assert_ne!(FlexBasis::absolute(0.0), FlexBasis::AUTO);
    assert_ne!(FlexBasis::new(0.0, FlexBasisKind::Relative), FlexBasis::AUTO);
}

/// Every text form parses back to an equal basis.
#[test]
fn additional_to_string_round_trips_through_parse() {
    for (value, kind, _) in TO_STRING_ROWS {
        let basis = FlexBasis::new(*value, *kind);
        let text = basis.to_string();
        let parsed = FlexBasis::parse(&text).ok();
        // The percent form allows digits and a decimal point only: a
        // percentage whose text is in scientific notation does not parse.
        if basis.is_relative() && text.contains('E') {
            assert_eq!(parsed, None, "{text}");
        } else if basis.is_relative() {
            let parsed = parsed.unwrap_or_else(|| panic!("{text} parses"));
            assert!(parsed.is_relative());
            assert!((parsed.value() - basis.value()).abs() <= basis.value() * 1e-15, "{text}");
        } else {
            assert_eq!(parsed, Some(basis), "{text}");
        }
    }
}

const TO_STRING_ROWS: &[(f64, FlexBasisKind, &str)] = &[
    (0.0, FlexBasisKind::Auto, "Auto"),
    (0.0, FlexBasisKind::Absolute, "0"),
    (-0.0, FlexBasisKind::Absolute, "-0"),
    (0.1, FlexBasisKind::Absolute, "0.10000000000000001"),
    (100.0, FlexBasisKind::Absolute, "100"),
    (1e16, FlexBasisKind::Absolute, "10000000000000000"),
    (1e17, FlexBasisKind::Absolute, "1E+17"),
    (123456789012345678.0, FlexBasisKind::Absolute, "1.2345678901234568E+17"),
    (1e20, FlexBasisKind::Absolute, "1E+20"),
    (0.0001, FlexBasisKind::Absolute, "0.0001"),
    (0.00001, FlexBasisKind::Absolute, "1.0000000000000001E-05"),
    (0.000012345, FlexBasisKind::Absolute, "1.2345E-05"),
    (1.0 / 3.0, FlexBasisKind::Absolute, "0.33333333333333331"),
    (2.5e-7, FlexBasisKind::Absolute, "2.4999999999999999E-07"),
    (f64::MAX, FlexBasisKind::Absolute, "1.7976931348623157E+308"),
    (5e-324, FlexBasisKind::Absolute, "4.9406564584124654E-324"),
    (12345.678, FlexBasisKind::Absolute, "12345.678"),
    (0.5, FlexBasisKind::Relative, "50%"),
    (0.3, FlexBasisKind::Relative, "30%"),
    (0.07, FlexBasisKind::Relative, "7.0000000000000009%"),
];

const fn relative(value: f64) -> Option<FlexBasis> {
    Some(FlexBasis::from_parts(value, FlexBasisKind::Relative))
}

const fn absolute(value: f64) -> Option<FlexBasis> {
    Some(FlexBasis::from_parts(value, FlexBasisKind::Absolute))
}

const PARSE_ROWS: &[(&str, Option<FlexBasis>)] = &[
    ("AUTO", Some(FlexBasis::AUTO)),
    // "auto" is compared with the untrimmed text.
    (" auto", None),
    ("auto ", None),
    (" 50% ", relative(0.5)),
    ("50 %", None),
    ("12.5%", relative(0.125)),
    ("-5%", None),
    ("+5%", None),
    ("1e2%", None),
    (".5%", relative(0.005)),
    ("5.%", relative(0.05)),
    ("%", None),
    (" 2 ", absolute(2.0)),
    ("-2", None),
    ("-0", absolute(-0.0)),
    ("1e2", absolute(100.0)),
    ("", None),
    ("NaN", None),
    ("Infinity", None),
    ("1,000", None),
    ("0x10", None),
    ("Infinity%", None),
    ("NaN%", None),
];
