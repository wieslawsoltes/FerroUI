//! Tests specific to this port: the properties of a class can be found by
//! name once the class is initialised on the thread, without any property
//! accessor having been called first (the equivalent of the static field
//! initialisers of the class having run). Creating an instance initialises
//! the class; so does a lookup into a class that declares its properties in
//! a `ferro_properties!` block (see `class_registration_tests.rs`).
//!
//! The tests must not call a property accessor: each test runs on its own
//! thread and so starts without any registration.

use crate::input::InputElement;
use crate::interactivity::Interactive;
use crate::layout::Layoutable;
use crate::visual::Visual;
use crate::{FerroPropertyRegistry, StyledElement, TypeInfo};

const STYLED_ELEMENT: &[&str] = &["DataContext", "Name", "Parent", "TemplatedParent", "Theme"];

const VISUAL: &[&str] = &[
    "Bounds",
    "ClipToBounds",
    "Clip",
    "IsVisible",
    "Opacity",
    "OpacityMask",
    "CacheMode",
    "Effect",
    "HasMirrorTransform",
    "RenderTransform",
    "RenderTransformOrigin",
    "FlowDirection",
    "VisualParent",
    "ZIndex",
];

const LAYOUTABLE: &[&str] = &[
    "DesiredSize",
    "Width",
    "Height",
    "MinWidth",
    "MaxWidth",
    "MinHeight",
    "MaxHeight",
    "Margin",
    "HorizontalAlignment",
    "VerticalAlignment",
    "UseLayoutRounding",
];

const INPUT_ELEMENT: &[&str] = &[
    "Focusable",
    "IsEnabled",
    "IsEffectivelyEnabled",
    "Cursor",
    "IsKeyboardFocusWithin",
    "IsFocused",
    "IsHitTestVisible",
    "IsPointerOver",
    "IsTabStop",
    "TabIndex",
    "IsHoldingEnabled",
    "IsHoldWithMouseEnabled",
];

fn assert_registered(type_: &'static TypeInfo, names: &[&[&str]]) {
    let registry = FerroPropertyRegistry::instance();
    for name in names.iter().flat_map(|names| names.iter()) {
        assert!(registry.find_registered(type_, name).is_some(), "{type_}.{name} is not registered");
    }
}

fn assert_not_registered(type_: &'static TypeInfo, names: &[&str]) {
    let registry = FerroPropertyRegistry::instance();
    for name in names {
        assert!(registry.find_registered(type_, name).is_none(), "{type_}.{name} is registered");
    }
}

#[test]
fn styled_element_properties_are_registered_by_construction() {
    let _target = StyledElement::new();
    assert_registered(StyledElement::TYPE, &[STYLED_ELEMENT]);
}

#[test]
fn visual_properties_are_registered_by_construction() {
    let _target = Visual::new();
    assert_registered(Visual::TYPE, &[STYLED_ELEMENT, VISUAL]);
    assert_not_registered(Visual::TYPE, LAYOUTABLE);
}

#[test]
fn visual_properties_are_found_before_any_instance_exists() {
    // The lookup initialises the class and its base classes.
    assert_registered(Visual::TYPE, &[STYLED_ELEMENT, VISUAL]);
    let declared: Vec<_> =
        FerroPropertyRegistry::instance().get_declared(Visual::TYPE).iter().map(|p| p.name().to_string()).collect();
    assert_eq!(declared.first().map(String::as_str), Some("Bounds"));
    assert_eq!(declared.last().map(String::as_str), Some("ZIndex"));
    // Declared with the theme variant type, owned by the styled element.
    assert_registered(StyledElement::TYPE, &[&["ActualThemeVariant", "RequestedThemeVariant"]]);
}

#[test]
fn visual_attached_property_is_registered_on_its_host_by_construction() {
    let _target = Visual::new();
    let attached = FerroPropertyRegistry::instance().get_registered_attached(Visual::TYPE);
    assert!(attached.iter().any(|p| p.name() == "FlowDirection"));
}

#[test]
fn layoutable_properties_are_registered_by_construction() {
    let _target = Layoutable::new();
    assert_registered(Layoutable::TYPE, &[STYLED_ELEMENT, VISUAL, LAYOUTABLE]);
}

#[test]
fn layoutable_direct_properties_are_registered_by_construction() {
    let _target = Layoutable::new();
    let direct = FerroPropertyRegistry::instance().get_registered_direct(Layoutable::TYPE);
    for name in ["DesiredSize", "Bounds", "HasMirrorTransform", "VisualParent"] {
        assert!(direct.iter().any(|p| p.name() == name), "{name} is not registered as direct");
    }
}

#[test]
fn layoutable_properties_are_found_before_any_instance_exists() {
    assert_registered(Layoutable::TYPE, &[LAYOUTABLE]);
    let direct = FerroPropertyRegistry::instance().get_registered_direct(Layoutable::TYPE);
    assert!(direct.iter().any(|p| p.name() == "DesiredSize"));
}

#[test]
fn interactive_properties_are_registered_by_construction() {
    // The class defines no property of its own.
    let _target = Interactive::new();
    assert_registered(Interactive::TYPE, &[STYLED_ELEMENT, VISUAL, LAYOUTABLE]);
    assert_not_registered(Interactive::TYPE, INPUT_ELEMENT);
}

#[test]
fn input_element_properties_are_registered_by_construction() {
    let _target = InputElement::new();
    assert_registered(InputElement::TYPE, &[STYLED_ELEMENT, VISUAL, LAYOUTABLE, INPUT_ELEMENT]);
}

#[test]
fn input_element_attached_properties_are_registered_on_their_host_by_construction() {
    let _target = InputElement::new();
    let attached = FerroPropertyRegistry::instance().get_registered_attached(StyledElement::TYPE);
    for name in ["IsHoldingEnabled", "IsHoldWithMouseEnabled"] {
        assert!(attached.iter().any(|p| p.name() == name), "{name} is not registered as attached");
    }
}

#[test]
fn input_element_properties_are_found_before_any_instance_exists() {
    assert_registered(InputElement::TYPE, &[INPUT_ELEMENT]);
    // Attached property owners that cannot be instantiated are found by
    // owner type and name, and have a namespace.
    let registry = FerroPropertyRegistry::instance();
    assert!(registry.find_registered(crate::input::KeyboardNavigation::TYPE, "TabNavigation").is_some());
    assert!(registry.find_registered(crate::input::navigation::XYFocus::TYPE, "Left").is_some());
    crate::register_types();
    assert!(TypeInfo::find("FerroUI.Input", "KeyboardNavigation").is_some());
    assert!(TypeInfo::find("FerroUI.Input.TextInput", "TextInputOptions").is_some());
}
