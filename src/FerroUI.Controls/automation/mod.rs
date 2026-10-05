//! UI automation (accessibility): the attached properties that describe an
//! element to assistive technology, the automation peers and the provider
//! contracts the peers implement.

mod automation_element_identifiers;
mod automation_live_setting;
mod automation_properties;
mod automation_property;
mod automation_property_changed_event_args;
mod element_not_enabled_exception;
mod expand_collapse_pattern_identifiers;
mod expand_collapse_state;
mod is_offscreen_behavior;
pub mod peers;
pub mod provider;
mod range_value_pattern_identifiers;
mod scroll_pattern_identifiers;
mod selection_item_pattern_identifiers;
mod selection_pattern_identifiers;
mod toggle_pattern_identifiers;
mod value_pattern_identifiers;

#[cfg(test)]
mod automation_properties_tests;

pub use automation_element_identifiers::AutomationElementIdentifiers;
pub use automation_live_setting::AutomationLiveSetting;
pub use automation_properties::{AccessibilityView, AutomationProperties};
pub use automation_property::AutomationProperty;
pub use automation_property_changed_event_args::AutomationPropertyChangedEventArgs;
pub use element_not_enabled_exception::ElementNotEnabledException;
pub use expand_collapse_pattern_identifiers::ExpandCollapsePatternIdentifiers;
pub use expand_collapse_state::ExpandCollapseState;
pub use is_offscreen_behavior::IsOffscreenBehavior;
pub use range_value_pattern_identifiers::RangeValuePatternIdentifiers;
pub use scroll_pattern_identifiers::ScrollPatternIdentifiers;
pub use selection_item_pattern_identifiers::SelectionItemPatternIdentifiers;
pub use selection_pattern_identifiers::SelectionPatternIdentifiers;
pub use toggle_pattern_identifiers::TogglePatternIdentifiers;
pub use value_pattern_identifiers::ValuePatternIdentifiers;
