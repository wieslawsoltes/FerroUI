//! Not from upstream: the upstream project has no tests of the converters.
//! These check the conversions against the behaviour of the original
//! converters.

use super::*;
use crate::primitives::converters::{AccentColorConverter, ContrastBrushConverter};
use crate::AlphaComponentPosition;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingOperations;
use ferroui_base::media::{Color, Colors, HsvColor, IBrush, SolidColorBrush};
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

fn boxed<T: ferroui_base::PropertyValue>(value: T) -> BoxedValue {
    Rc::new(value)
}

fn brush_color(value: Option<BoxedValue>) -> Color {
    let brush = value.and_then(|value| value.downcast_ref::<Rc<dyn IBrush>>().cloned()).expect("a brush");
    brush.as_solid_color_brush().expect("a solid color brush").color()
}

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|value| Rc::ptr_eq(value, &FerroProperty::unset_value()))
}

#[test]
fn to_hex_string_places_the_alpha_component() {
    let color = Color::from_argb(0x80, 0x12, 0x34, 0x56);

    assert_eq!("80123456", ColorToHexConverter::to_hex_string(color, AlphaComponentPosition::Leading, true, false));
    assert_eq!("12345680", ColorToHexConverter::to_hex_string(color, AlphaComponentPosition::Trailing, true, false));
    assert_eq!("#123456", ColorToHexConverter::to_hex_string(color, AlphaComponentPosition::Trailing, false, true));
}

#[test]
fn parse_hex_string_reads_every_form() {
    let leading = AlphaComponentPosition::Leading;
    let trailing = AlphaComponentPosition::Trailing;

    assert_eq!(Some(Color::from_argb(0xFF, 0x12, 0x34, 0x56)), ColorToHexConverter::parse_hex_string("123456", leading));
    assert_eq!(Some(Color::from_argb(0x80, 0x12, 0x34, 0x56)), ColorToHexConverter::parse_hex_string(" #80123456 ", leading));
    assert_eq!(Some(Color::from_argb(0x80, 0x12, 0x34, 0x56)), ColorToHexConverter::parse_hex_string("12345680", trailing));
    assert_eq!(Some(Color::from_argb(0xFF, 0xAA, 0xBB, 0xCC)), ColorToHexConverter::parse_hex_string("#ABC", leading));
    assert_eq!(Some(Color::from_argb(0xDD, 0xAA, 0xBB, 0xCC)), ColorToHexConverter::parse_hex_string("ABCD", trailing));
    assert_eq!(None, ColorToHexConverter::parse_hex_string("12345", leading));
    assert_eq!(None, ColorToHexConverter::parse_hex_string("GG3456", leading));
}

#[test]
fn color_to_hex_converter_converts_both_ways() {
    let converter = ColorToHexConverter::new();
    converter.set_is_alpha_visible(false);

    let text = converter.convert(Some(&boxed(Colors::RED)), ValueType::of::<String>(), Some(&boxed(true))).unwrap();
    assert_eq!(Some("#FF0000".to_string()), text.and_then(|value| value.downcast_ref::<String>().cloned()));

    let color = converter.convert_back(Some(&boxed("00FF00".to_string())), ValueType::of::<Color>(), None).unwrap();
    assert_eq!(Some(Colors::LIME), color.and_then(|value| value.downcast_ref::<Color>().copied()));

    assert!(is_unset(&converter.convert_back(Some(&boxed("x".to_string())), ValueType::of::<Color>(), None).unwrap()));
    assert!(is_unset(&converter.convert(Some(&boxed(1)), ValueType::of::<String>(), None).unwrap()));
}

#[test]
fn accent_color_converter_steps_the_value_component() {
    let converter = AccentColorConverter::new();
    let base = HsvColor::new(1.0, 120.0, 1.0, 0.5);

    let lighter = converter.convert(Some(&boxed(base)), ValueType::of::<Rc<dyn IBrush>>(), Some(&boxed("1".to_string())));
    assert_eq!(HsvColor::new(1.0, 120.0, 1.0, 0.6).to_rgb(), brush_color(lighter.unwrap()));

    let darker = converter.convert(Some(&boxed(base)), ValueType::of::<Rc<dyn IBrush>>(), Some(&boxed(-2)));
    assert_eq!(HsvColor::new(1.0, 120.0, 1.0, 0.3).to_rgb(), brush_color(darker.unwrap()));

    let invalid = converter.convert(Some(&boxed(base)), ValueType::of::<Rc<dyn IBrush>>(), Some(&boxed("a".to_string())));
    assert!(is_unset(&invalid.unwrap()));
    assert_eq!(base, AccentColorConverter::get_accent(base, 0));
}

#[test]
fn contrast_brush_converter_chooses_black_or_white() {
    let converter = ContrastBrushConverter::new();
    let target = ValueType::of::<Rc<dyn IBrush>>();

    assert_eq!(Colors::BLACK, brush_color(converter.convert(Some(&boxed(Colors::YELLOW)), target, None).unwrap()));
    assert_eq!(Colors::WHITE, brush_color(converter.convert(Some(&boxed(Colors::NAVY)), target, None).unwrap()));

    // Below the alpha threshold the default color of the parameter is used.
    let transparent = Color::from_argb(0x10, 0, 0, 0);
    let result = converter.convert(Some(&boxed(transparent)), target, Some(&boxed(Colors::RED))).unwrap();
    assert_eq!(Colors::RED, brush_color(result));
}

#[test]
fn to_color_converter_applies_the_opacity_of_a_brush() {
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color_and_opacity(Colors::RED, 0.5).into();

    let color = ToColorConverter::new().convert(Some(&boxed(brush)), ValueType::of::<Color>(), None).unwrap();

    assert_eq!(Some(Color::from_argb(127, 255, 0, 0)), color.and_then(|value| value.downcast_ref::<Color>().copied()));
}

#[test]
fn to_brush_converter_converts_colors() {
    let converter = ToBrushConverter::new();
    let target = ValueType::of::<Rc<dyn IBrush>>();

    assert_eq!(Colors::BLUE, brush_color(converter.convert(Some(&boxed(Colors::BLUE)), target, None).unwrap()));
    assert_eq!(Colors::RED, brush_color(converter.convert(Some(&boxed(Colors::RED.to_hsv())), target, None).unwrap()));
    assert!(is_unset(&converter.convert(Some(&boxed("red".to_string())), target, None).unwrap()));
}

#[test]
fn color_to_display_name_converter_names_opaque_colors() {
    let converter = ColorToDisplayNameConverter::new();
    let target = ValueType::of::<String>();

    let name = converter.convert(Some(&boxed(Colors::RED)), target, None).unwrap();
    assert_eq!(Some("Red".to_string()), name.and_then(|value| value.downcast_ref::<String>().cloned()));
    assert!(is_unset(&converter.convert(Some(&boxed(Colors::TRANSPARENT)), target, None).unwrap()));
}

#[test]
fn do_nothing_for_null_converter_passes_values_through() {
    let converter = DoNothingForNullConverter::new();
    let value = boxed(5);

    let converted = converter.convert(Some(&value), ValueType::of::<i32>(), None).unwrap().unwrap();
    assert!(Rc::ptr_eq(&value, &converted));

    let nothing = converter.convert(None, ValueType::of::<i32>(), None).unwrap().unwrap();
    assert!(Rc::ptr_eq(&nothing, &BindingOperations::do_nothing()));
}
