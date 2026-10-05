use super::peers::{AutomationControlType, AutomationLandmarkType};
use super::{AutomationLiveSetting, IsOffscreenBehavior};
use crate::Control;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, ElementRef, FerroProperty, Nullable, Ref, StyledElement,
};

/// Declares how a control should included in different views of the automation tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AccessibilityView {
    /// The control's view is defined by its automation peer.
    Default,

    /// The control is included in the Raw view of the automation tree.
    Raw,

    /// The control is included in the Control view of the automation tree.
    Control,

    /// The control is included in the Content view of the automation tree.
    Content,
}

/// The attached properties that describe an element to UI automation.
pub struct AutomationProperties;

ferro_static_type!(AutomationProperties);

ferro_properties! {
    impl AutomationProperties {
        /// Defines the AutomationProperties.AcceleratorKey attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_accelerator_key`.
        pub fn accelerator_key_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("AcceleratorKey", None)
        }

        /// Defines the AutomationProperties.AccessibilityView attached property.
        ///
        /// The value of this property affects the default value of the
        /// `AutomationPeer::is_content_element` and `AutomationPeer::is_control_element` properties.
        pub fn accessibility_view_property() -> AttachedProperty<AccessibilityView> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("AccessibilityView", AccessibilityView::Default)
        }

        /// Defines the AutomationProperties.AccessKey attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_access_key`.
        pub fn access_key_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("AccessKey", None)
        }

        /// Defines the AutomationProperties.AutomationId attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_automation_id`.
        pub fn automation_id_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("AutomationId", None)
        }

        /// Defines the AutomationProperties.ControlTypeOverride attached property.
        ///
        /// This property affects the default value for
        /// `AutomationPeer::get_automation_control_type`.
        pub fn control_type_override_property() -> AttachedProperty<Option<AutomationControlType>> {
            ValueTypes::register_nullable::<AutomationControlType>();
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("ControlTypeOverride", None)
        }

        /// Defines the AutomationProperties.ClassNameOverride attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_class_name`.
        pub fn class_name_override_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("ClassNameOverride", None)
        }

        /// Defines the AutomationProperties.IsControlElementOverride attached property.
        ///
        /// This property affects the default value for
        /// `AutomationPeer::is_control_element`.
        pub fn is_control_element_override_property() -> AttachedProperty<Option<bool>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("IsControlElementOverride", None)
        }

        /// Defines the AutomationProperties.HelpText attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_help_text`.
        pub fn help_text_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("HelpText", None)
        }

        /// Defines the AutomationProperties.LandmarkType attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_landmark_type`
        pub fn landmark_type_property() -> AttachedProperty<Option<AutomationLandmarkType>> {
            ValueTypes::register_nullable::<AutomationLandmarkType>();
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("LandmarkType", None)
        }

        /// Defines the AutomationProperties.HeadingLevel attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_heading_level`.
        pub fn heading_level_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("HeadingLevel", 0)
        }

        /// Defines the AutomationProperties.IsColumnHeader attached property.
        ///
        /// This property currently has no effect.
        pub fn is_column_header_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("IsColumnHeader", false)
        }

        /// Defines the AutomationProperties.IsRequiredForForm attached property.
        ///
        /// This property currently has no effect.
        pub fn is_required_for_form_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("IsRequiredForForm", false)
        }

        /// Defines the AutomationProperties.IsRowHeader attached property.
        ///
        /// This property currently has no effect.
        pub fn is_row_header_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("IsRowHeader", false)
        }

        /// Defines the AutomationProperties.IsOffscreenBehavior attached property.
        ///
        /// This property affects the default value for `AutomationPeer::is_offscreen`.
        pub fn is_offscreen_behavior_property() -> AttachedProperty<IsOffscreenBehavior> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("IsOffscreenBehavior", IsOffscreenBehavior::Default)
        }

        /// Defines the AutomationProperties.ItemStatus attached property.
        ///
        /// This property currently has no effect.
        pub fn item_status_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("ItemStatus", None)
        }

        /// Defines the AutomationProperties.ItemType attached property.
        ///
        /// This property currently has no effect.
        pub fn item_type_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("ItemType", None)
        }

        /// Defines the AutomationProperties.LabeledBy attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_labeled_by`.
        ///
        /// An element does not own the control that labels it, so the value is
        /// an element reference.
        pub fn labeled_by_property() -> AttachedProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("LabeledBy", None)
        }

        /// Defines the AutomationProperties.LiveSetting attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_live_setting` and controls whether live region changed events are emitted.
        pub fn live_setting_property() -> AttachedProperty<AutomationLiveSetting> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("LiveSetting", AutomationLiveSetting::Off)
        }

        /// Defines the AutomationProperties.Name attached property.
        ///
        /// This property affects the default value for `AutomationPeer::get_name`.
        pub fn name_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("Name", None)
        }

        /// Defines the AutomationProperties.PositionInSet attached property.
        ///
        /// NOTE: This property currently has no effect.
        ///
        /// The PositionInSet property describes the ordinal location of the element within a set
        /// of elements which are considered to be siblings. PositionInSet works in coordination
        /// with the SizeOfSet property to describe the ordinal location in the set.
        pub fn position_in_set_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("PositionInSet", AutomationProperties::AUTOMATION_POSITION_IN_SET_DEFAULT)
        }

        /// Defines the AutomationProperties.SizeOfSet attached property.
        ///
        /// NOTE: This property currently has no effect.
        ///
        /// The SizeOfSet property describes the count of automation elements in a group or set
        /// that are considered to be siblings. SizeOfSet works in coordination with the PositionInSet
        /// property to describe the count of items in the set.
        pub fn size_of_set_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<AutomationProperties, StyledElement, _>("SizeOfSet", AutomationProperties::AUTOMATION_SIZE_OF_SET_DEFAULT)
        }
    }
}

impl AutomationProperties {
    pub(crate) const AUTOMATION_POSITION_IN_SET_DEFAULT: i32 = -1;
    pub(crate) const AUTOMATION_SIZE_OF_SET_DEFAULT: i32 = -1;

    /// Helper for setting the value of the [`accelerator_key_property`](Self::accelerator_key_property) on a StyledElement.
    pub fn set_accelerator_key(element: &StyledElement, value: &str) {
        element.set_value(Self::accelerator_key_property(), Some(value.to_owned()))
    }

    /// Helper for reading the value of the [`accelerator_key_property`](Self::accelerator_key_property) on a StyledElement.
    pub fn get_accelerator_key(element: &StyledElement) -> Option<String> {
        element.get_value(Self::accelerator_key_property())
    }

    /// Helper for setting the value of the [`accessibility_view_property`](Self::accessibility_view_property) on a StyledElement.
    pub fn set_accessibility_view(element: &StyledElement, value: AccessibilityView) {
        element.set_value(Self::accessibility_view_property(), value)
    }

    /// Helper for reading the value of the [`accessibility_view_property`](Self::accessibility_view_property) on a StyledElement.
    pub fn get_accessibility_view(element: &StyledElement) -> AccessibilityView {
        element.get_value(Self::accessibility_view_property())
    }

    /// Helper for setting the value of the [`access_key_property`](Self::access_key_property) on a StyledElement.
    pub fn set_access_key(element: &StyledElement, value: &str) {
        element.set_value(Self::access_key_property(), Some(value.to_owned()))
    }

    /// Helper for reading the value of the [`access_key_property`](Self::access_key_property) on a StyledElement.
    pub fn get_access_key(element: &StyledElement) -> Option<String> {
        element.get_value(Self::access_key_property())
    }

    /// Helper for setting the value of the [`automation_id_property`](Self::automation_id_property) on a StyledElement.
    pub fn set_automation_id(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::automation_id_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`automation_id_property`](Self::automation_id_property) on a StyledElement.
    pub fn get_automation_id(element: &StyledElement) -> Option<String> {
        element.get_value(Self::automation_id_property())
    }

    /// Helper for setting the value of the [`control_type_override_property`](Self::control_type_override_property) on a StyledElement.
    pub fn set_control_type_override(element: &StyledElement, value: Option<AutomationControlType>) {
        element.set_value(Self::control_type_override_property(), value)
    }

    /// Helper for reading the value of the [`control_type_override_property`](Self::control_type_override_property) on a StyledElement.
    pub fn get_control_type_override(element: &StyledElement) -> Option<AutomationControlType> {
        element.get_value(Self::control_type_override_property())
    }

    /// Helper for setting the value of the [`class_name_override_property`](Self::class_name_override_property) on a StyledElement.
    pub fn set_class_name_override(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::class_name_override_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`class_name_override_property`](Self::class_name_override_property) on a StyledElement.
    pub fn get_class_name_override(element: &StyledElement) -> Option<String> {
        element.get_value(Self::class_name_override_property())
    }

    /// Helper for setting the value of the [`is_control_element_override_property`](Self::is_control_element_override_property) on a StyledElement.
    pub fn set_is_control_element_override(element: &StyledElement, value: Option<bool>) {
        element.set_value(Self::is_control_element_override_property(), value)
    }

    /// Helper for reading the value of the [`is_control_element_override_property`](Self::is_control_element_override_property) on a StyledElement.
    pub fn get_is_control_element_override(element: &StyledElement) -> Option<bool> {
        element.get_value(Self::is_control_element_override_property())
    }

    /// Helper for setting the value of the [`help_text_property`](Self::help_text_property) on a StyledElement.
    pub fn set_help_text(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::help_text_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`help_text_property`](Self::help_text_property) on a StyledElement.
    pub fn get_help_text(element: &StyledElement) -> Option<String> {
        element.get_value(Self::help_text_property())
    }

    /// Helper for setting the value of the [`landmark_type_property`](Self::landmark_type_property) on a StyledElement.
    pub fn set_landmark_type(element: &StyledElement, value: Option<AutomationLandmarkType>) {
        element.set_value(Self::landmark_type_property(), value)
    }

    /// Helper for reading the value of the [`landmark_type_property`](Self::landmark_type_property) on a StyledElement.
    pub fn get_landmark_type(element: &StyledElement) -> Option<AutomationLandmarkType> {
        element.get_value(Self::landmark_type_property())
    }

    /// Helper for setting the value of the [`heading_level_property`](Self::heading_level_property) on a StyledElement.
    pub fn set_heading_level(element: &StyledElement, value: i32) {
        element.set_value(Self::heading_level_property(), value)
    }

    /// Helper for reading the value of the [`heading_level_property`](Self::heading_level_property) on a StyledElement.
    pub fn get_heading_level(element: &StyledElement) -> i32 {
        element.get_value(Self::heading_level_property())
    }

    /// Helper for setting the value of the [`is_column_header_property`](Self::is_column_header_property) on a StyledElement.
    pub fn set_is_column_header(element: &StyledElement, value: bool) {
        element.set_value(Self::is_column_header_property(), value)
    }

    /// Helper for reading the value of the [`is_column_header_property`](Self::is_column_header_property) on a StyledElement.
    pub fn get_is_column_header(element: &StyledElement) -> bool {
        element.get_value(Self::is_column_header_property())
    }

    /// Helper for setting the value of the [`is_required_for_form_property`](Self::is_required_for_form_property) on a StyledElement.
    pub fn set_is_required_for_form(element: &StyledElement, value: bool) {
        element.set_value(Self::is_required_for_form_property(), value)
    }

    /// Helper for reading the value of the [`is_required_for_form_property`](Self::is_required_for_form_property) on a StyledElement.
    pub fn get_is_required_for_form(element: &StyledElement) -> bool {
        element.get_value(Self::is_required_for_form_property())
    }

    /// Helper for reading the value of the [`is_row_header_property`](Self::is_row_header_property) on a StyledElement.
    pub fn get_is_row_header(element: &StyledElement) -> bool {
        element.get_value(Self::is_row_header_property())
    }

    /// Helper for setting the value of the [`is_row_header_property`](Self::is_row_header_property) on a StyledElement.
    pub fn set_is_row_header(element: &StyledElement, value: bool) {
        element.set_value(Self::is_row_header_property(), value)
    }

    /// Helper for setting the value of the [`is_offscreen_behavior_property`](Self::is_offscreen_behavior_property) on a StyledElement.
    pub fn set_is_offscreen_behavior(element: &StyledElement, value: IsOffscreenBehavior) {
        element.set_value(Self::is_offscreen_behavior_property(), value)
    }

    /// Helper for reading the value of the [`is_offscreen_behavior_property`](Self::is_offscreen_behavior_property) on a StyledElement.
    pub fn get_is_offscreen_behavior(element: &StyledElement) -> IsOffscreenBehavior {
        element.get_value(Self::is_offscreen_behavior_property())
    }

    /// Helper for setting the value of the [`item_status_property`](Self::item_status_property) on a StyledElement.
    pub fn set_item_status(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::item_status_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`item_status_property`](Self::item_status_property) on a StyledElement.
    pub fn get_item_status(element: &StyledElement) -> Option<String> {
        element.get_value(Self::item_status_property())
    }

    /// Helper for setting the value of the [`item_type_property`](Self::item_type_property) on a StyledElement.
    pub fn set_item_type(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::item_type_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`item_type_property`](Self::item_type_property) on a StyledElement.
    pub fn get_item_type(element: &StyledElement) -> Option<String> {
        element.get_value(Self::item_type_property())
    }

    /// Helper for setting the value of the [`labeled_by_property`](Self::labeled_by_property) on a StyledElement.
    pub fn set_labeled_by(element: &StyledElement, value: impl Into<Nullable<Control>>) {
        element.set_value(Self::labeled_by_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Helper for reading the value of the [`labeled_by_property`](Self::labeled_by_property) on a StyledElement.
    pub fn get_labeled_by(element: &StyledElement) -> Option<Ref<Control>> {
        ElementRef::resolve(&element.get_value(Self::labeled_by_property()))
    }

    /// Helper for setting the value of the [`live_setting_property`](Self::live_setting_property) on a StyledElement.
    pub fn set_live_setting(element: &StyledElement, value: AutomationLiveSetting) {
        element.set_value(Self::live_setting_property(), value)
    }

    /// Helper for reading the value of the [`live_setting_property`](Self::live_setting_property) on a StyledElement.
    pub fn get_live_setting(element: &StyledElement) -> AutomationLiveSetting {
        element.get_value(Self::live_setting_property())
    }

    /// Helper for setting the value of the [`name_property`](Self::name_property) on a StyledElement.
    pub fn set_name(element: &StyledElement, value: Option<&str>) {
        element.set_value(Self::name_property(), value.map(str::to_owned))
    }

    /// Helper for reading the value of the [`name_property`](Self::name_property) on a StyledElement.
    pub fn get_name(element: &StyledElement) -> Option<String> {
        element.get_value(Self::name_property())
    }

    /// Helper for setting the value of the [`position_in_set_property`](Self::position_in_set_property) on a StyledElement.
    pub fn set_position_in_set(element: &StyledElement, value: i32) {
        element.set_value(Self::position_in_set_property(), value)
    }

    /// Helper for reading the value of the [`position_in_set_property`](Self::position_in_set_property) on a StyledElement.
    pub fn get_position_in_set(element: &StyledElement) -> i32 {
        element.get_value(Self::position_in_set_property())
    }

    /// Helper for setting the value of the [`size_of_set_property`](Self::size_of_set_property) on a StyledElement.
    pub fn set_size_of_set(element: &StyledElement, value: i32) {
        element.set_value(Self::size_of_set_property(), value)
    }

    /// Helper for reading the value of the [`size_of_set_property`](Self::size_of_set_property) on a StyledElement.
    pub fn get_size_of_set(element: &StyledElement) -> i32 {
        element.get_value(Self::size_of_set_property())
    }
}
