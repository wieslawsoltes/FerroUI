//! The converters of the crate (`converters`): `CornerRadiusToThicknessConverter`, every
//! conversion kind and the multiplier, as `CornerRadiusToThicknessConverter.cpp` of
//! microsoft-ui-xaml computes them (microsoft-ui-xaml has no tests of the converter), its markup
//! metadata, and the six resources of `CornerRadius_themeresources.xaml` that are instances of it.

use super::support::*;
use crate::converters::{CornerRadiusToThicknessConverter, CornerRadiusToThicknessConverterKind as Kind};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, MarkupType};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, CornerRadius, FerroProperty, Thickness};
use std::rc::Rc;

/// A radius whose corners are told apart: top left 1, top right 2, bottom right 3, bottom left 4.
const RADIUS: CornerRadius = CornerRadius::new(1.0, 2.0, 3.0, 4.0);

fn convert(converter: &CornerRadiusToThicknessConverter, value: BoxedValue) -> Option<BoxedValue> {
    converter
        .convert(Some(&value), ValueType::of::<Thickness>(), None, &CultureInfo::invariant_culture())
        .expect("a conversion")
}

fn thickness(value: Option<BoxedValue>) -> Thickness {
    value.expect("a value").downcast_ref::<Thickness>().copied().expect("a thickness")
}

/// Every kind with the thickness it makes of `RADIUS` (left, top, right, bottom).
const KINDS: [(Kind, Thickness); 12] = [
    (Kind::FilterTopAndBottomFromLeft, Thickness::new(0.0, 1.0, 0.0, 4.0)),
    (Kind::FilterTopAndBottomFromRight, Thickness::new(0.0, 2.0, 0.0, 3.0)),
    (Kind::FilterLeftAndRightFromTop, Thickness::new(1.0, 0.0, 2.0, 0.0)),
    (Kind::FilterLeftAndRightFromBottom, Thickness::new(4.0, 0.0, 3.0, 0.0)),
    (Kind::FilterTopFromTopLeft, Thickness::new(0.0, 1.0, 0.0, 0.0)),
    (Kind::FilterTopFromTopRight, Thickness::new(0.0, 2.0, 0.0, 0.0)),
    (Kind::FilterRightFromTopRight, Thickness::new(0.0, 0.0, 2.0, 0.0)),
    (Kind::FilterRightFromBottomRight, Thickness::new(0.0, 0.0, 3.0, 0.0)),
    (Kind::FilterBottomFromBottomRight, Thickness::new(0.0, 0.0, 0.0, 3.0)),
    (Kind::FilterBottomFromBottomLeft, Thickness::new(0.0, 0.0, 0.0, 4.0)),
    (Kind::FilterLeftFromBottomLeft, Thickness::new(4.0, 0.0, 0.0, 0.0)),
    (Kind::FilterLeftFromTopLeft, Thickness::new(1.0, 0.0, 0.0, 0.0)),
];

#[test]
fn the_defaults_are_the_ones_of_winui() {
    let converter = CornerRadiusToThicknessConverter::new();
    assert_eq!(Kind::FilterLeftAndRightFromTop, converter.conversion_kind());
    assert_eq!(1.0, converter.multiplier());
    assert_eq!(Thickness::new(1.0, 0.0, 2.0, 0.0), thickness(convert(&converter, Rc::new(RADIUS))));
}

#[test]
fn every_conversion_kind_takes_its_corners() {
    let converter = CornerRadiusToThicknessConverter::new();
    for (kind, expected) in KINDS {
        converter.set_conversion_kind(kind);
        assert_eq!(kind, converter.conversion_kind());
        assert_eq!(expected, thickness(convert(&converter, Rc::new(RADIUS))), "{kind:?}");
        assert_eq!(expected, CornerRadiusToThicknessConverter::convert_radius(RADIUS, kind, 1.0), "{kind:?}");
    }
}

#[test]
fn the_multiplier_scales_every_side() {
    let converter = CornerRadiusToThicknessConverter::new();
    for multiplier in [2.0, 0.5, -1.0, 0.0] {
        converter.set_multiplier(multiplier);
        assert_eq!(multiplier, converter.multiplier());
        for (kind, expected) in KINDS {
            converter.set_conversion_kind(kind);
            let scaled = Thickness::new(
                expected.left * multiplier,
                expected.top * multiplier,
                expected.right * multiplier,
                expected.bottom * multiplier,
            );
            assert_eq!(scaled, thickness(convert(&converter, Rc::new(RADIUS))), "{kind:?} * {multiplier}");
        }
    }

    // The inset corners of TabView: the bottom corners, negated.
    let radius = CornerRadius::new(8.0, 8.0, 4.0, 4.0);
    assert_eq!(
        Thickness::new(-4.0, 0.0, 0.0, 0.0),
        CornerRadiusToThicknessConverter::convert_radius(radius, Kind::FilterLeftFromBottomLeft, -1.0)
    );
    assert_eq!(
        Thickness::new(0.0, 0.0, -4.0, 0.0),
        CornerRadiusToThicknessConverter::convert_radius(radius, Kind::FilterRightFromBottomRight, -1.0)
    );
}

#[test]
fn other_values_convert_to_the_unset_value_and_there_is_no_way_back() {
    let converter = CornerRadiusToThicknessConverter::new();
    let unset = |value: Option<BoxedValue>| value.is_some_and(|value| Rc::ptr_eq(&value, &FerroProperty::unset_value()));
    assert!(unset(convert(&converter, Rc::new(5i32))));
    assert!(unset(converter.convert(None, ValueType::object(), None, &CultureInfo::invariant_culture()).unwrap()));
    assert!(converter
        .convert_back(Some(&(Rc::new(Thickness::uniform(1.0)) as BoxedValue)), ValueType::object(), None, &CultureInfo::invariant_culture())
        .is_err());
}

#[test]
fn the_converter_is_created_and_configured_from_markup() {
    crate::register_types();
    let find = |name: &str| {
        MarkupType::find("FerroUI.Themes.MacOS.Converters", name).unwrap_or_else(|| panic!("{name} has no markup metadata"))
    };
    let markup = find("CornerRadiusToThicknessConverter");
    assert_eq!(12, find("CornerRadiusToThicknessConverterKind").enum_members.len());

    let converter = (markup.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
    assert!(from_markup_value::<Rc<dyn IValueConverter>>(&converter).is_some());
    let kind: BoxedValue = Rc::new(Kind::FilterTopFromTopRight);
    (markup.find_property("ConversionKind").unwrap().set.unwrap())(&[converter.clone(), Some(kind)]).unwrap();
    let multiplier: BoxedValue = Rc::new(3.0f64);
    (markup.find_property("Multiplier").unwrap().set.unwrap())(&[converter.clone(), Some(multiplier)]).unwrap();
    let concrete = from_markup_value::<Rc<CornerRadiusToThicknessConverter>>(&converter).expect("the converter");
    assert_eq!(Kind::FilterTopFromTopRight, concrete.conversion_kind());
    assert_eq!(3.0, concrete.multiplier());
}

/// The six resources of `CornerRadius_themeresources.xaml`, as the theme defines them.
#[test]
fn the_thickness_converters_of_the_theme() {
    let _app = start_application();
    let theme = crate::MacOSTheme::new();
    let converter = |key: &str| {
        let value = theme.try_get_resource(&ResourceKey::from(key), None).unwrap_or_else(|| panic!("{key} is not defined"));
        from_markup_value::<Rc<CornerRadiusToThicknessConverter>>(&value).unwrap_or_else(|| panic!("{key} is not the converter"))
    };
    for (key, kind, multiplier) in [
        ("TopThicknessFilterConverter", Kind::FilterLeftAndRightFromTop, 1.0),
        ("BottomThicknessFilterConverter", Kind::FilterLeftAndRightFromBottom, 1.0),
        ("LeftThicknessFilterConverter", Kind::FilterTopAndBottomFromLeft, 1.0),
        ("RightThicknessFilterConverter", Kind::FilterTopAndBottomFromRight, 1.0),
        ("TabViewLeftInsetCornerConverter", Kind::FilterLeftFromBottomLeft, -1.0),
        ("TabViewRightInsetCornerConverter", Kind::FilterRightFromBottomRight, -1.0),
    ] {
        let converter = converter(key);
        assert_eq!(kind, converter.conversion_kind(), "{key}");
        assert_eq!(multiplier, converter.multiplier(), "{key}");
    }
}
