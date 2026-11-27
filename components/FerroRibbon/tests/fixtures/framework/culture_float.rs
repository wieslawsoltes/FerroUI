//! SDK-only culture Float parser consumer; no FerroRibbon imports.
use ferroui_base::utilities::{NumberFormatInfo, number_format::try_parse_double_float};
#[test]
fn culture_float_grammar_is_not_invariant_or_decimal_replacement() {
    let info = NumberFormatInfo::new()
        .with_number_decimal_separator(",")
        .with_positive_sign("plus")
        .with_negative_sign("minus");
    assert_eq!(
        try_parse_double_float(Some("minus1,5eplus2"), &info),
        Some(-150.)
    );
    assert!(try_parse_double_float(Some("1.5"), &info).is_none());
    assert!(try_parse_double_float(Some("1,234,5"), &info).is_none());
    assert!(try_parse_double_float(None, &info).is_none());
}
