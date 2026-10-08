//! Not from upstream: what the markup metadata of the crate states.

use crate::register_types;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::metadata::{from_markup_value, MarkupType};
use ferroui_base::TypeInfo;
use std::rc::Rc;

#[test]
fn converters_are_known_by_their_namespaces_and_construct() {
    register_types();

    for (namespace, name) in [
        ("FerroUI.Controls.Converters", "ColorToDisplayNameConverter"),
        ("FerroUI.Controls.Converters", "ColorToHexConverter"),
        ("FerroUI.Controls.Converters", "DoNothingForNullConverter"),
        ("FerroUI.Controls.Converters", "ToBrushConverter"),
        ("FerroUI.Controls.Converters", "ToColorConverter"),
        ("FerroUI.Controls.Primitives.Converters", "AccentColorConverter"),
        ("FerroUI.Controls.Primitives.Converters", "ContrastBrushConverter"),
    ] {
        let markup = MarkupType::find(namespace, name).unwrap_or_else(|| panic!("{namespace}.{name}"));
        let instance = (markup.constructors[0].invoke)(&[]).expect("a converter");
        assert!(from_markup_value::<Rc<dyn IValueConverter>>(&instance).is_some(), "{name}");
    }
}

#[test]
fn palettes_and_enumerations_are_known() {
    register_types();

    for name in [
        "FlatColorPalette",
        "FlatHalfColorPalette",
        "FluentColorPalette",
        "MaterialColorPalette",
        "MaterialHalfColorPalette",
        "SixteenColorPalette",
        "IColorPalette",
        "ColorModel",
        "ColorComponent",
        "ColorSpectrumShape",
        "ColorSpectrumComponents",
        "AlphaComponentPosition",
        "ColorViewTab",
        "HsvComponent",
        "RgbComponent",
    ] {
        assert!(MarkupType::find("FerroUI.Controls", name).is_some(), "{name}");
    }
    assert!(MarkupType::find("FerroUI.Controls.Primitives", "ColorHelper").is_some());
}

#[test]
fn the_classes_state_their_content_and_template_parts() {
    register_types();

    let picker = TypeInfo::find("FerroUI.Controls", "ColorPicker").expect("ColorPicker");
    assert_eq!(Some("Content"), picker.markup().and_then(|markup| markup.content_property));

    let spectrum = TypeInfo::find("FerroUI.Controls.Primitives", "ColorSpectrum").expect("ColorSpectrum");
    assert!(spectrum.markup().is_some_and(|markup| !markup.attributes.is_empty()));
}
