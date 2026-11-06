//! Measured CLR Enum.TryParse bridge shared by native attached properties and theme JSON.
use ferroboard_core::text::{TextUtils, Utf16Text, ordinal_ignore_case_equal};
fn trim(mut units: &[u16]) -> &[u16] {
    while units.first().is_some_and(|u| TextUtils::is_white_space(*u)) {
        units = &units[1..];
    }
    while units.last().is_some_and(|u| TextUtils::is_white_space(*u)) {
        units = &units[..units.len() - 1];
    }
    units
}
pub(crate) fn parse_enum(text: Option<&Utf16Text>, names: &[&str]) -> Option<i32> {
    parse_enum_with_case(text, names, true)
}
pub(crate) fn parse_enum_case_sensitive(text: Option<&Utf16Text>, names: &[&str]) -> Option<i32> {
    parse_enum_with_case(text, names, false)
}
fn parse_enum_with_case(
    text: Option<&Utf16Text>,
    names: &[&str],
    ignore_case: bool,
) -> Option<i32> {
    let mut text = text?.units();
    while text.first().is_some_and(|u| TextUtils::is_white_space(*u)) {
        text = &text[1..];
    }
    if text
        .first()
        .is_some_and(|u| matches!(*u, 43 | 45 | 48..=57))
    {
        let mut numeric = text;
        while numeric.last() == Some(&0) {
            numeric = &numeric[..numeric.len() - 1];
        }
        while numeric.last().is_some_and(|u| matches!(*u, 9..=13 | 32)) {
            numeric = &numeric[..numeric.len() - 1];
        }
        if let Ok(s) = String::from_utf16(numeric)
            && let Ok(v) = s.parse::<i32>()
        {
            return Some(v);
        }
    }
    let mut combined = 0;
    for part in text.split(|u| *u == 44) {
        let part = Utf16Text::from_units(trim(part));
        let index = names.iter().position(|name| {
            if ignore_case {
                ordinal_ignore_case_equal(&part, &(*name).into())
            } else {
                part == Utf16Text::from(*name)
            }
        })?;
        combined |= index as i32;
    }
    Some(combined)
}
