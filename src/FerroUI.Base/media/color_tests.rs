//! Port of `Media/ColorTests.cs`.
//!
//! Where two upstream names differ only in the case of a format specifier
//! (`HslColor_ToString_X_Converts_To_Rgb_Hex` and
//! `HslColor_ToString_x_Converts_To_Rgb_Hex`, and the same pair of `HsvColor`),
//! the upper-case one keeps its capital letter, so both keep upstream's name.
//!
//! `Parse_Throws_ArgumentNullException_For_Null_Input` and the `null` row of
//! `TryParse_Returns_False_For_Invalid_Input` have no counterpart: the string
//! argument of `Color::parse` and `Color::try_parse` cannot be null.

use crate::media::{Color, HslColor, HsvColor};
use crate::utilities::CultureInfo;

fn invariant_culture() -> CultureInfo {
    CultureInfo::invariant_culture()
}

#[test]
fn parse_parses_rgb_hash_color() {
    let result = Color::parse("#ff8844").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn try_parse_parses_rgb_hash_color() {
    let result = Color::try_parse("#ff8844");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn parse_parses_rgb_hash_shorthand_color() {
    let result = Color::parse("#f84").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn try_parse_parses_rgb_hash_shorthand_color() {
    let result = Color::try_parse("#f84");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn parse_parses_argb_hash_color() {
    let result = Color::parse("#40ff8844").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0x40, result.a);
}

#[test]
fn try_parse_parses_argb_hash_color() {
    let result = Color::try_parse("#40ff8844");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0x40, result.a);
}

#[test]
fn parse_parses_argb_hash_shorthand_color() {
    let result = Color::parse("#4f84").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0x44, result.a);
}

#[test]
fn try_parse_parses_argb_hash_shorthand_color() {
    let result = Color::try_parse("#4f84");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x88, result.g);
    assert_eq!(0x44, result.b);
    assert_eq!(0x44, result.a);
}

#[test]
fn parse_parses_named_color_lowercase() {
    let result = Color::parse("red").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x00, result.g);
    assert_eq!(0x00, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn try_parse_parses_named_color_lowercase() {
    let result = Color::try_parse("red");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x00, result.g);
    assert_eq!(0x00, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn parse_parses_named_color_uppercase() {
    let result = Color::parse("RED").unwrap();

    assert_eq!(0xff, result.r);
    assert_eq!(0x00, result.g);
    assert_eq!(0x00, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn try_parse_parses_named_color_uppercase() {
    let result = Color::try_parse("RED");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(0xff, result.r);
    assert_eq!(0x00, result.g);
    assert_eq!(0x00, result.b);
    assert_eq!(0xff, result.a);
}

#[test]
fn parse_hex_value_doesnt_accept_too_few_chars() {
    assert!(Color::parse("#ff").is_err());
}

#[test]
fn try_parse_hex_value_doesnt_accept_too_few_chars() {
    assert!(Color::try_parse("#ff").is_none());
}

#[test]
fn parse_hex_value_doesnt_accept_too_many_chars() {
    assert!(Color::parse("#ff5555555").is_err());
}

#[test]
fn try_parse_hex_value_doesnt_accept_too_many_chars() {
    assert!(Color::try_parse("#ff5555555").is_none());
}

#[test]
fn parse_hex_value_doesnt_accept_invalid_number() {
    assert!(Color::parse("#ff808g80").is_err());
}

#[test]
fn try_parse_hex_value_doesnt_accept_invalid_number() {
    assert!(Color::try_parse("#ff808g80").is_none());
}

#[test]
fn parse_throws_format_exception_for_invalid_input() {
    assert!(Color::parse("").is_err());
}

#[test]
fn try_parse_returns_false_for_invalid_input() {
    for input in [""] {
        assert!(Color::try_parse(input).is_none(), "{input:?}");
    }
}

#[test]
fn try_parse_hsl_color() {
    // Inline data requires constants, so the data is handled internally here
    let data: [(&str, HslColor); 24] = [
        // HSV
        ("hsl(0, 0, 0)", HslColor::new(1.0, 0.0, 0.0, 0.0)),
        ("hsl(0, 0%, 0%)", HslColor::new(1.0, 0.0, 0.0, 0.0)),
        ("hsl(180, 0.5, 0.5)", HslColor::new(1.0, 180.0, 0.5, 0.5)),
        ("hsl(180, 50%, 50%)", HslColor::new(1.0, 180.0, 0.5, 0.5)),
        ("hsl(360, 1.0, 1.0)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsl(360, 100%, 100%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsl(-1000, -1000, -1000)", HslColor::new(1.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsl(-1000, -1000%, -1000%)", HslColor::new(1.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsl(1000, 1000, 1000)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsl(1000, 1000%, 1000%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsl(300, 0.8, 0.2)", HslColor::new(1.0, 300.0, 0.8, 0.2)),
        ("hsl(300, 80%, 20%)", HslColor::new(1.0, 300.0, 0.8, 0.2)),
        // HSVA
        ("hsla(0, 0, 0, 0)", HslColor::new(0.0, 0.0, 0.0, 0.0)),
        ("hsla(0, 0%, 0%, 0%)", HslColor::new(0.0, 0.0, 0.0, 0.0)),
        ("hsla(180, 0.5, 0.5, 0.5)", HslColor::new(0.5, 180.0, 0.5, 0.5)),
        ("hsla(180, 50%, 50%, 50%)", HslColor::new(0.5, 180.0, 0.5, 0.5)),
        ("hsla(360, 1.0, 1.0, 1.0)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsla(360, 100%, 100%, 100%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsla(-1000, -1000, -1000, -1000)", HslColor::new(0.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsla(-1000, -1000%, -1000%, -1000%)", HslColor::new(0.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsla(1000, 1000, 1000, 1000)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsla(1000, 1000%, 1000%, 1000%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsla(300, 0.9, 0.2, 0.8)", HslColor::new(0.8, 300.0, 0.9, 0.2)),
        ("hsla(300, 90%, 20%, 0.8)", HslColor::new(0.8, 300.0, 0.9, 0.2)),
    ];

    for data_point in data {
        let parsed_hsl_color = HslColor::try_parse(data_point.0);
        assert!(parsed_hsl_color.is_some(), "{}", data_point.0);
        assert!(data_point.1 == parsed_hsl_color.unwrap(), "{}", data_point.0);
    }
}

#[test]
fn try_parse_hsv_color() {
    // Inline data requires constants, so the data is handled internally here
    let data: [(&str, HsvColor); 24] = [
        // HSV
        ("hsv(0, 0, 0)", HsvColor::new(1.0, 0.0, 0.0, 0.0)),
        ("hsv(0, 0%, 0%)", HsvColor::new(1.0, 0.0, 0.0, 0.0)),
        ("hsv(180, 0.5, 0.5)", HsvColor::new(1.0, 180.0, 0.5, 0.5)),
        ("hsv(180, 50%, 50%)", HsvColor::new(1.0, 180.0, 0.5, 0.5)),
        ("hsv(360, 1.0, 1.0)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsv(360, 100%, 100%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsv(-1000, -1000, -1000)", HsvColor::new(1.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsv(-1000, -1000%, -1000%)", HsvColor::new(1.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsv(1000, 1000, 1000)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsv(1000, 1000%, 1000%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsv(300, 0.8, 0.2)", HsvColor::new(1.0, 300.0, 0.8, 0.2)),
        ("hsv(300, 80%, 20%)", HsvColor::new(1.0, 300.0, 0.8, 0.2)),
        // HSVA
        ("hsva(0, 0, 0, 0)", HsvColor::new(0.0, 0.0, 0.0, 0.0)),
        ("hsva(0, 0%, 0%, 0%)", HsvColor::new(0.0, 0.0, 0.0, 0.0)),
        ("hsva(180, 0.5, 0.5, 0.5)", HsvColor::new(0.5, 180.0, 0.5, 0.5)),
        ("hsva(180, 50%, 50%, 50%)", HsvColor::new(0.5, 180.0, 0.5, 0.5)),
        ("hsva(360, 1.0, 1.0, 1.0)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsva(360, 100%, 100%, 100%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
        ("hsva(-1000, -1000, -1000, -1000)", HsvColor::new(0.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsva(-1000, -1000%, -1000%, -1000%)", HsvColor::new(0.0, 0.0, 0.0, 0.0)), // Clamps to min
        ("hsva(1000, 1000, 1000, 1000)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsva(1000, 1000%, 1000%, 1000%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
        ("hsva(300, 0.9, 0.2, 0.8)", HsvColor::new(0.8, 300.0, 0.9, 0.2)),
        ("hsva(300, 90%, 20%, 0.8)", HsvColor::new(0.8, 300.0, 0.9, 0.2)),
    ];

    for data_point in data {
        let parsed_hsv_color = HsvColor::try_parse(data_point.0);
        assert!(parsed_hsv_color.is_some(), "{}", data_point.0);
        assert!(data_point.1 == parsed_hsv_color.unwrap(), "{}", data_point.0);
    }
}

#[test]
fn try_parse_all_formats_with_conversion() {
    // Inline data requires constants, so the data is handled internally here
    let data: [(&str, Color); 20] = [
        // RGB
        ("White", Color::new(0xff, 0xff, 0xff, 0xff)),
        ("#123456", Color::new(0xff, 0x12, 0x34, 0x56)),
        ("rgb(100, 30, 45)", Color::new(255, 100, 30, 45)),
        ("rgba(100, 30, 45, 0.9)", Color::new(230, 100, 30, 45)),
        ("rgba(100, 30, 45, 90%)", Color::new(230, 100, 30, 45)),
        ("rgb(255,0,0)", Color::new(255, 255, 0, 0)),
        ("rgb(0,255,0)", Color::new(255, 0, 255, 0)),
        ("rgb(0,0,255)", Color::new(255, 0, 0, 255)),
        ("rgb(100%, 0, 0)", Color::new(255, 255, 0, 0)),
        ("rgb(0, 100%, 0)", Color::new(255, 0, 255, 0)),
        ("rgb(0, 0, 100%)", Color::new(255, 0, 0, 255)),
        ("rgba(0, 0, 100%, 50%)", Color::new(128, 0, 0, 255)),
        ("rgba(50%, 10%, 80%, 50%)", Color::new(128, 128, 26, 204)),
        ("rgba(50%, 10%, 80%, 0.5)", Color::new(128, 128, 26, 204)),
        // HSL
        ("hsl(296, 85%, 12%)", Color::new(255, 53, 5, 57)),
        ("hsla(296, 0.85, 0.12, 0.9)", Color::new(230, 53, 5, 57)),
        ("hsla(296, 85%, 12%, 90%)", Color::new(230, 53, 5, 57)),
        // HSV
        ("hsv(240, 83%, 78%)", Color::new(255, 34, 34, 199)),
        ("hsva(240, 0.83, 0.78, 0.9)", Color::new(230, 34, 34, 199)),
        ("hsva(240, 83%, 78%, 90%)", Color::new(230, 34, 34, 199)),
    ];

    for data_point in data {
        let parsed_color = Color::try_parse(data_point.0);
        assert!(parsed_color.is_some(), "{}", data_point.0);
        assert!(data_point.1 == parsed_color.unwrap(), "{}", data_point.0);
    }
}

#[test]
fn hsv_to_from_hsl_conversion() {
    // Note that conversion of values more representative of actual colors is not done due to rounding error
    // It would be necessary to introduce a different equality comparison that accounts for rounding differences in values
    // This is a result of the math in the conversion itself
    // RGB doesn't have this problem because it uses whole numbers
    let data: [(HsvColor, HslColor); 6] = [
        (HsvColor::new(1.0, 0.0, 0.0, 0.0), HslColor::new(1.0, 0.0, 0.0, 0.0)),
        (HsvColor::new(1.0, 359.0, 1.0, 1.0), HslColor::new(1.0, 359.0, 1.0, 0.5)),
        (HsvColor::new(1.0, 128.0, 0.0, 0.0), HslColor::new(1.0, 128.0, 0.0, 0.0)),
        (HsvColor::new(1.0, 128.0, 0.0, 1.0), HslColor::new(1.0, 128.0, 0.0, 1.0)),
        (HsvColor::new(1.0, 128.0, 1.0, 1.0), HslColor::new(1.0, 128.0, 1.0, 0.5)),
        (HsvColor::new(0.23, 0.5, 1.0, 1.0), HslColor::new(0.23, 0.5, 1.0, 0.5)),
    ];

    for data_point in data {
        let converted_hsl = data_point.0.to_hsl();
        let converted_hsv = data_point.1.to_hsv();

        assert_eq!(converted_hsv, data_point.0);
        assert_eq!(converted_hsl, data_point.1);
    }
}

// =====================================================================
// IFormattable unified format specifier tests
//
// All three color types (Color, HslColor, HsvColor) support ALL
// format specifiers. Cross-model formats auto-convert.
//
// Convention:
//   Uppercase = include alpha, a-suffixed prefix (rgba, hsla, hsva)
//   Lowercase = exclude alpha, plain prefix (rgb, hsl, hsv)
//   "%" suffix = percent mode
// =====================================================================

#[test]
fn color_to_string_default_returns_known_name() {
    let red = Color::new(0xFF, 0xFF, 0x00, 0x00);

    assert_eq!("Red", red.to_string());
}

#[test]
fn color_to_string_default_returns_hex_for_unknown() {
    let color = Color::new(0x40, 0xFF, 0x88, 0x44);

    assert_eq!("#40ff8844", color.to_string());
}

#[test]
fn color_to_string_null_format_matches_default() {
    let color = Color::new(0x40, 0xFF, 0x88, 0x44);

    assert_eq!(color.to_string(), color.to_string_format(None, None).unwrap());
}

#[test]
fn color_to_string_empty_format_matches_default() {
    let color = Color::new(0x40, 0xFF, 0x88, 0x44);

    assert_eq!(color.to_string(), color.to_string_format(Some(""), None).unwrap());
}

#[test]
fn hsl_color_to_string_null_format_matches_default() {
    let color = HslColor::new(0.8, 200.0, 0.6, 0.4);

    assert_eq!(color.to_string(), color.to_string_format(None, None).unwrap());
}

#[test]
fn hsv_color_to_string_null_format_matches_default() {
    let color = HsvColor::new(0.8, 200.0, 0.6, 0.4);

    assert_eq!(color.to_string(), color.to_string_format(None, None).unwrap());
}

#[test]
fn color_to_string_x_returns_xaml_hex_with_alpha() {
    let data: [(u8, u8, u8, u8, &str); 4] = [
        (0xFF, 0xFF, 0x88, 0x44, "#FFFF8844"),
        (0x40, 0xFF, 0x88, 0x44, "#40FF8844"),
        (0xFF, 0x00, 0x00, 0x00, "#FF000000"),
        (0x00, 0x00, 0x00, 0x00, "#00000000"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("X"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_x_returns_hex_without_alpha() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x88, 0x44, "#FF8844"),
        (0x40, 0xFF, 0x88, 0x44, "#FF8844"),
        (0x00, 0x00, 0x00, 0x00, "#000000"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("x"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_h_returns_html_hex_with_alpha() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x88, 0x44, "#FF8844FF"),
        (0x40, 0xFF, 0x88, 0x44, "#FF884440"),
        (0x00, 0x00, 0x00, 0x00, "#00000000"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("H"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
#[allow(non_snake_case)]
fn hsl_color_to_string_X_converts_to_rgb_hex() {
    // Pure red: HSL(0, 1, 0.5) = RGB(255, 0, 0)
    let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);

    assert_eq!("#FFFF0000", hsl.to_string_format(Some("X"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsl_color_to_string_x_converts_to_rgb_hex() {
    let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);

    assert_eq!("#FF0000", hsl.to_string_format(Some("x"), Some(&invariant_culture())).unwrap());
}

#[test]
#[allow(non_snake_case)]
fn hsv_color_to_string_X_converts_to_rgb_hex() {
    // Pure red: HSV(0, 1, 1) = RGB(255, 0, 0)
    let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);

    assert_eq!("#FFFF0000", hsv.to_string_format(Some("X"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsv_color_to_string_x_converts_to_rgb_hex() {
    let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);

    assert_eq!("#FF0000", hsv.to_string_format(Some("x"), Some(&invariant_culture())).unwrap());
}

#[test]
fn color_to_string_r_returns_rgba_with_alpha() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x88, 0x44, "rgba(255, 136, 68, 1.00)"),
        (0x80, 0xFF, 0x88, 0x44, "rgba(255, 136, 68, 0.50)"),
        (0x00, 0x00, 0x00, 0x00, "rgba(0, 0, 0, 0.00)"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("R"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_r_returns_rgb_without_alpha() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x88, 0x44, "rgb(255, 136, 68)"),
        (0x80, 0xFF, 0x88, 0x44, "rgb(255, 136, 68)"),
        (0xFF, 0x00, 0x00, 0x00, "rgb(0, 0, 0)"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("r"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsl_color_to_string_r_converts_to_rgba() {
    // Pure blue: HSL(240, 1, 0.5) = RGB(0, 0, 255)
    let hsl = HslColor::new(1.0, 240.0, 1.0, 0.5);

    assert_eq!("rgba(0, 0, 255, 1.00)", hsl.to_string_format(Some("R"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsv_color_to_string_r_converts_to_rgb() {
    // Pure red: HSV(0, 1, 1) = RGB(255, 0, 0)
    let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);

    assert_eq!("rgb(255, 0, 0)", hsv.to_string_format(Some("r"), Some(&invariant_culture())).unwrap());
}

#[test]
fn color_to_string_r_pct_returns_rgba_percent() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x80, 0x00, "rgba(100%, 50%, 0%, 100%)"),
        (0x80, 0xFF, 0x80, 0x00, "rgba(100%, 50%, 0%, 50%)"),
        (0x00, 0x00, 0x00, 0x00, "rgba(0%, 0%, 0%, 0%)"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("R%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_r_pct_returns_rgb_percent() {
    let data: [(u8, u8, u8, u8, &str); 3] = [
        (0xFF, 0xFF, 0x80, 0x00, "rgb(100%, 50%, 0%)"),
        (0x80, 0xFF, 0x80, 0x00, "rgb(100%, 50%, 0%)"),
        (0xFF, 0x00, 0x00, 0x00, "rgb(0%, 0%, 0%)"),
    ];

    for (a, r, g, b, expected) in data {
        let color = Color::new(a, r, g, b);

        assert_eq!(expected, color.to_string_format(Some("r%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsl_color_to_string_r_pct_converts_to_rgba_percent() {
    // Pure red: HSL(0, 1, 0.5) = RGB(255, 0, 0)
    let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);

    assert_eq!("rgba(100%, 0%, 0%, 100%)", hsl.to_string_format(Some("R%"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsl_color_to_string_l_returns_hsla_with_alpha() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsla(180, 50%, 50%, 1.00)"),
        (0.5, 240.0, 0.8, 0.2, "hsla(240, 80%, 20%, 0.50)"),
        (0.0, 0.0, 0.0, 0.0, "hsla(0, 0%, 0%, 0.00)"),
    ];

    for (a, h, s, l, expected) in data {
        let color = HslColor::new(a, h, s, l);

        assert_eq!(expected, color.to_string_format(Some("L"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsl_color_to_string_l_returns_hsl_without_alpha() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsl(180, 50%, 50%)"),
        (0.5, 240.0, 0.8, 0.2, "hsl(240, 80%, 20%)"),
        (0.0, 0.0, 0.0, 0.0, "hsl(0, 0%, 0%)"),
    ];

    for (a, h, s, l, expected) in data {
        let color = HslColor::new(a, h, s, l);

        assert_eq!(expected, color.to_string_format(Some("l"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_l_converts_to_hsla() {
    // Pure red: RGB(255, 0, 0) = HSL(0, 100%, 50%)
    let color = Color::new(0xFF, 0xFF, 0x00, 0x00);

    assert_eq!("hsla(0, 100%, 50%, 1.00)", color.to_string_format(Some("L"), Some(&invariant_culture())).unwrap());
}

#[test]
fn color_to_string_l_converts_to_hsl() {
    let color = Color::new(0xFF, 0xFF, 0x00, 0x00);

    assert_eq!("hsl(0, 100%, 50%)", color.to_string_format(Some("l"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsv_color_to_string_l_converts_to_hsla() {
    // Pure red: HSV(0, 1, 1) = HSL(0, 100%, 50%)
    let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);

    assert_eq!("hsla(0, 100%, 50%, 1.00)", hsv.to_string_format(Some("L"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsl_color_to_string_l_pct_returns_hsla_all_percent() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsla(50%, 50%, 50%, 100%)"),
        (0.5, 90.0, 1.0, 1.0, "hsla(25%, 100%, 100%, 50%)"),
        (1.0, 0.0, 0.0, 0.0, "hsla(0%, 0%, 0%, 100%)"),
    ];

    for (a, h, s, l, expected) in data {
        let color = HslColor::new(a, h, s, l);

        assert_eq!(expected, color.to_string_format(Some("L%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsl_color_to_string_l_pct_returns_hsl_all_percent() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsl(50%, 50%, 50%)"),
        (0.5, 90.0, 1.0, 1.0, "hsl(25%, 100%, 100%)"),
        (1.0, 0.0, 0.0, 0.0, "hsl(0%, 0%, 0%)"),
    ];

    for (a, h, s, l, expected) in data {
        let color = HslColor::new(a, h, s, l);

        assert_eq!(expected, color.to_string_format(Some("l%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsv_color_to_string_v_returns_hsva_with_alpha() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsva(180, 50%, 50%, 1.00)"),
        (0.5, 240.0, 0.8, 0.2, "hsva(240, 80%, 20%, 0.50)"),
        (0.0, 0.0, 0.0, 0.0, "hsva(0, 0%, 0%, 0.00)"),
    ];

    for (a, h, s, v, expected) in data {
        let color = HsvColor::new(a, h, s, v);

        assert_eq!(expected, color.to_string_format(Some("V"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsv_color_to_string_v_returns_hsv_without_alpha() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsv(180, 50%, 50%)"),
        (0.5, 240.0, 0.8, 0.2, "hsv(240, 80%, 20%)"),
        (0.0, 0.0, 0.0, 0.0, "hsv(0, 0%, 0%)"),
    ];

    for (a, h, s, v, expected) in data {
        let color = HsvColor::new(a, h, s, v);

        assert_eq!(expected, color.to_string_format(Some("v"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_v_converts_to_hsva() {
    // Pure red: RGB(255, 0, 0) = HSV(0, 100%, 100%)
    let color = Color::new(0xFF, 0xFF, 0x00, 0x00);

    assert_eq!("hsva(0, 100%, 100%, 1.00)", color.to_string_format(Some("V"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsl_color_to_string_v_converts_to_hsva() {
    // Pure red: HSL(0, 1, 0.5) = HSV(0, 100%, 100%)
    let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);

    assert_eq!("hsva(0, 100%, 100%, 1.00)", hsl.to_string_format(Some("V"), Some(&invariant_culture())).unwrap());
}

#[test]
fn hsv_color_to_string_v_pct_returns_hsva_all_percent() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsva(50%, 50%, 50%, 100%)"),
        (0.5, 90.0, 1.0, 1.0, "hsva(25%, 100%, 100%, 50%)"),
        (1.0, 0.0, 0.0, 0.0, "hsva(0%, 0%, 0%, 100%)"),
    ];

    for (a, h, s, v, expected) in data {
        let color = HsvColor::new(a, h, s, v);

        assert_eq!(expected, color.to_string_format(Some("V%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn hsv_color_to_string_v_pct_returns_hsv_all_percent() {
    let data: [(f64, f64, f64, f64, &str); 3] = [
        (1.0, 180.0, 0.5, 0.5, "hsv(50%, 50%, 50%)"),
        (0.5, 90.0, 1.0, 1.0, "hsv(25%, 100%, 100%)"),
        (1.0, 0.0, 0.0, 0.0, "hsv(0%, 0%, 0%)"),
    ];

    for (a, h, s, v, expected) in data {
        let color = HsvColor::new(a, h, s, v);

        assert_eq!(expected, color.to_string_format(Some("v%"), Some(&invariant_culture())).unwrap());
    }
}

#[test]
fn color_to_string_invalid_format_throws() {
    let color = Color::new(0xFF, 0xFF, 0x00, 0x00);

    assert!(color.to_string_format(Some("Z"), None).is_err());
}

#[test]
fn hsl_color_to_string_invalid_format_throws() {
    let color = HslColor::new(1.0, 0.0, 0.0, 0.0);

    assert!(color.to_string_format(Some("Z"), None).is_err());
}

#[test]
fn hsv_color_to_string_invalid_format_throws() {
    let color = HsvColor::new(1.0, 0.0, 0.0, 0.0);

    assert!(color.to_string_format(Some("Z"), None).is_err());
}

#[test]
fn color_to_string_reserved_and_removed_specifiers_throw() {
    for format in ["C", "c", "A", "a", "P", "h"] {
        let color = Color::new(0xFF, 0xFF, 0x00, 0x00);

        assert!(color.to_string_format(Some(format), None).is_err(), "{format}");
    }
}

#[test]
fn hsl_color_to_string_reserved_c_throws() {
    for format in ["C", "c"] {
        let color = HslColor::new(1.0, 0.0, 1.0, 0.5);

        assert!(color.to_string_format(Some(format), None).is_err(), "{format}");
    }
}

#[test]
fn hsv_color_to_string_reserved_c_throws() {
    for format in ["C", "c"] {
        let color = HsvColor::new(1.0, 0.0, 1.0, 1.0);

        assert!(color.to_string_format(Some(format), None).is_err(), "{format}");
    }
}

#[test]
fn color_to_string_i_format_provider_is_ignored() {
    let color = Color::new(0x80, 0xFF, 0x88, 0x44);
    let french = CultureInfo::get_culture_info("fr-FR");

    assert_eq!(
        color.to_string_format(Some("R"), Some(&invariant_culture())).unwrap(),
        color.to_string_format(Some("R"), Some(&french)).unwrap()
    );
}

#[test]
fn hsl_color_to_string_i_format_provider_is_ignored() {
    let color = HslColor::new(0.5, 180.0, 0.5, 0.5);
    let french = CultureInfo::get_culture_info("fr-FR");

    assert_eq!(
        color.to_string_format(Some("L"), Some(&invariant_culture())).unwrap(),
        color.to_string_format(Some("L"), Some(&french)).unwrap()
    );
}
