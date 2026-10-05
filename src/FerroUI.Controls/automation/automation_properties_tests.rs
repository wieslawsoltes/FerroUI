//! Tests of the automation attached properties. The reference has no test
//! class of its own for them (they are exercised through the automation
//! peer tests); these check the registration, the default values and the
//! helpers.

use super::peers::{AutomationControlType, AutomationLandmarkType};
use super::{AccessibilityView, AutomationLiveSetting, AutomationProperties, IsOffscreenBehavior};
use crate::{Border, Control};
use ferroui_base::{FerroPropertyRegistry, StyledElement};

const NAMES: &[&str] = &[
    "AcceleratorKey",
    "AccessibilityView",
    "AccessKey",
    "AutomationId",
    "ControlTypeOverride",
    "ClassNameOverride",
    "IsControlElementOverride",
    "HelpText",
    "LandmarkType",
    "HeadingLevel",
    "IsColumnHeader",
    "IsRequiredForForm",
    "IsRowHeader",
    "IsOffscreenBehavior",
    "ItemStatus",
    "ItemType",
    "LabeledBy",
    "LiveSetting",
    "Name",
    "PositionInSet",
    "SizeOfSet",
];

#[test]
fn properties_are_registered_by_name_as_attached() {
    let registry = FerroPropertyRegistry::instance();
    for name in NAMES {
        let property = registry
            .find_registered(AutomationProperties::TYPE, name)
            .unwrap_or_else(|| panic!("{name} is not registered"));
        assert!(property.is_attached(), "{name}");
        assert_eq!(property.owner_type(), AutomationProperties::TYPE, "{name}");
    }
    let attached = registry.get_registered_attached(StyledElement::TYPE);
    for name in NAMES {
        assert!(
            attached.iter().any(|p| p.name() == *name && p.owner_type() == AutomationProperties::TYPE),
            "{name} is not attached to styled elements"
        );
    }
}

#[test]
fn default_values() {
    let target = Border::new();
    let e: &StyledElement = &target;

    assert_eq!(AutomationProperties::get_accelerator_key(e), None);
    assert_eq!(AutomationProperties::get_accessibility_view(e), AccessibilityView::Default);
    assert_eq!(AutomationProperties::get_access_key(e), None);
    assert_eq!(AutomationProperties::get_automation_id(e), None);
    assert_eq!(AutomationProperties::get_control_type_override(e), None);
    assert_eq!(AutomationProperties::get_class_name_override(e), None);
    assert_eq!(AutomationProperties::get_is_control_element_override(e), None);
    assert_eq!(AutomationProperties::get_help_text(e), None);
    assert_eq!(AutomationProperties::get_landmark_type(e), None);
    assert_eq!(AutomationProperties::get_heading_level(e), 0);
    assert!(!AutomationProperties::get_is_column_header(e));
    assert!(!AutomationProperties::get_is_required_for_form(e));
    assert!(!AutomationProperties::get_is_row_header(e));
    assert_eq!(AutomationProperties::get_is_offscreen_behavior(e), IsOffscreenBehavior::Default);
    assert_eq!(AutomationProperties::get_item_status(e), None);
    assert_eq!(AutomationProperties::get_item_type(e), None);
    assert!(AutomationProperties::get_labeled_by(e).is_none());
    assert_eq!(AutomationProperties::get_live_setting(e), AutomationLiveSetting::Off);
    assert_eq!(AutomationProperties::get_name(e), None);
    assert_eq!(AutomationProperties::get_position_in_set(e), -1);
    assert_eq!(AutomationProperties::get_size_of_set(e), -1);
}

#[test]
fn helpers_set_and_get_values() {
    let target = Border::new();
    let e: &StyledElement = &target;

    AutomationProperties::set_accelerator_key(e, "Ctrl+S");
    AutomationProperties::set_accessibility_view(e, AccessibilityView::Content);
    AutomationProperties::set_access_key(e, "S");
    AutomationProperties::set_automation_id(e, Some("id"));
    AutomationProperties::set_control_type_override(e, Some(AutomationControlType::Button));
    AutomationProperties::set_class_name_override(e, Some("Class"));
    AutomationProperties::set_is_control_element_override(e, Some(false));
    AutomationProperties::set_help_text(e, Some("help"));
    AutomationProperties::set_landmark_type(e, Some(AutomationLandmarkType::Main));
    AutomationProperties::set_heading_level(e, 2);
    AutomationProperties::set_is_column_header(e, true);
    AutomationProperties::set_is_required_for_form(e, true);
    AutomationProperties::set_is_row_header(e, true);
    AutomationProperties::set_is_offscreen_behavior(e, IsOffscreenBehavior::FromClip);
    AutomationProperties::set_item_status(e, Some("status"));
    AutomationProperties::set_item_type(e, Some("type"));
    AutomationProperties::set_live_setting(e, AutomationLiveSetting::Assertive);
    AutomationProperties::set_name(e, Some("name"));
    AutomationProperties::set_position_in_set(e, 3);
    AutomationProperties::set_size_of_set(e, 7);

    assert_eq!(AutomationProperties::get_accelerator_key(e).as_deref(), Some("Ctrl+S"));
    assert_eq!(AutomationProperties::get_accessibility_view(e), AccessibilityView::Content);
    assert_eq!(AutomationProperties::get_access_key(e).as_deref(), Some("S"));
    assert_eq!(AutomationProperties::get_automation_id(e).as_deref(), Some("id"));
    assert_eq!(AutomationProperties::get_control_type_override(e), Some(AutomationControlType::Button));
    assert_eq!(AutomationProperties::get_class_name_override(e).as_deref(), Some("Class"));
    assert_eq!(AutomationProperties::get_is_control_element_override(e), Some(false));
    assert_eq!(AutomationProperties::get_help_text(e).as_deref(), Some("help"));
    assert_eq!(AutomationProperties::get_landmark_type(e), Some(AutomationLandmarkType::Main));
    assert_eq!(AutomationProperties::get_heading_level(e), 2);
    assert!(AutomationProperties::get_is_column_header(e));
    assert!(AutomationProperties::get_is_required_for_form(e));
    assert!(AutomationProperties::get_is_row_header(e));
    assert_eq!(AutomationProperties::get_is_offscreen_behavior(e), IsOffscreenBehavior::FromClip);
    assert_eq!(AutomationProperties::get_item_status(e).as_deref(), Some("status"));
    assert_eq!(AutomationProperties::get_item_type(e).as_deref(), Some("type"));
    assert_eq!(AutomationProperties::get_live_setting(e), AutomationLiveSetting::Assertive);
    assert_eq!(AutomationProperties::get_name(e).as_deref(), Some("name"));
    assert_eq!(AutomationProperties::get_position_in_set(e), 3);
    assert_eq!(AutomationProperties::get_size_of_set(e), 7);

    AutomationProperties::set_name(e, None);
    assert_eq!(AutomationProperties::get_name(e), None);
}

#[test]
fn labeled_by_is_an_element_reference() {
    let target = Border::new();
    let label = Border::new();
    let e: &StyledElement = &target;

    AutomationProperties::set_labeled_by(e, label.clone().upcast::<Control>());
    let read = AutomationProperties::get_labeled_by(e).expect("the label is alive");
    assert!(read == label.clone().upcast::<Control>());

    // The property does not keep the labelling control alive.
    drop(read);
    drop(label);
    assert!(AutomationProperties::get_labeled_by(e).is_none());

    AutomationProperties::set_labeled_by(e, None);
    assert!(AutomationProperties::get_labeled_by(e).is_none());
}
