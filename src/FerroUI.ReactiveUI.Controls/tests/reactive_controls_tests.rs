//! `ReactiveControlsTests` (`ReactiveControlsTests.cs`): the reactive control types and their type
//! hierarchy.
//!
//! The reference inspects the types by reflection; here the type descriptors of the class model
//! answer the same questions (the class exists, its base classes, its registered properties).

use crate::{ReactiveUserControlBase, ReactiveWindowBase, RoutedViewHost, ViewModelViewHost};
use ferroui_base::{FerroPropertyRegistry, StaticType, TypeInfo};
use ferroui_controls::{ContentControl, UserControl, Window};

/// Whether the class `type_` derives from `base` (`BaseType` walked up to `object`).
fn derives_from(type_: &'static TypeInfo, base: &'static TypeInfo) -> bool {
    !std::ptr::eq(type_, base) && base.is_assignable_from(type_)
}

/// Whether the class `type_` has a registered property named `name` (`GetProperty(name)`).
fn has_property(type_: &'static TypeInfo, name: &str) -> bool {
    FerroPropertyRegistry::instance().find_registered(type_, name).is_some()
}

#[test]
fn reactive_user_control_can_be_created_without_setup() {
    let control_type = <ReactiveUserControlBase as StaticType>::TYPE;
    assert_eq!(control_type.name(), "ReactiveUserControlBase");
}

#[test]
fn reactive_window_can_be_created_without_setup() {
    let window_type = <ReactiveWindowBase as StaticType>::TYPE;
    assert_eq!(window_type.name(), "ReactiveWindowBase");
}

#[test]
fn view_model_view_host_type_exists() {
    let host_type = <ViewModelViewHost as StaticType>::TYPE;
    assert_eq!(host_type.name(), "ViewModelViewHost");
}

#[test]
fn routed_view_host_type_exists() {
    let host_type = <RoutedViewHost as StaticType>::TYPE;
    assert_eq!(host_type.name(), "RoutedViewHost");
}

#[test]
fn reactive_user_control_inherits_from_user_control() {
    assert!(derives_from(<ReactiveUserControlBase as StaticType>::TYPE, <UserControl as StaticType>::TYPE));
}

#[test]
fn reactive_window_inherits_from_window() {
    assert!(derives_from(<ReactiveWindowBase as StaticType>::TYPE, <Window as StaticType>::TYPE));
}

#[test]
fn view_model_view_host_inherits_from_content_control() {
    assert!(derives_from(<ViewModelViewHost as StaticType>::TYPE, <ContentControl as StaticType>::TYPE));
}

#[test]
fn routed_view_host_inherits_from_content_control() {
    assert!(derives_from(<RoutedViewHost as StaticType>::TYPE, <ContentControl as StaticType>::TYPE));
}

// Not ported: ReactiveUserControl_IsGeneric - reflection (a generic type definition and its type
// arguments; here the typed form is a generic Rust handle, `ReactiveUserControl<VM>`).
// Not ported: ReactiveWindow_IsGeneric - reflection (as above, `ReactiveWindow<VM>`).

#[test]
fn view_model_view_host_has_expected_properties() {
    let host_type = <ViewModelViewHost as StaticType>::TYPE;

    assert!(has_property(host_type, "ViewModel"));
    assert!(has_property(host_type, "DefaultContent"));
    assert!(has_property(host_type, "ViewContract"));
}

#[test]
fn routed_view_host_has_expected_properties() {
    let host_type = <RoutedViewHost as StaticType>::TYPE;

    assert!(has_property(host_type, "DefaultContent"));
    assert!(has_property(host_type, "ViewContract"));
    assert!(has_property(host_type, "Router"));
}
