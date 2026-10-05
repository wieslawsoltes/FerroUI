use super::{InputElement, KeyboardNavigationMode};
use crate::{ferro_property, AttachedProperty, FerroProperty, Ref, StyledElement};

/// Defines attached properties that control keyboard navigation behaviour
/// for a container.
pub struct KeyboardNavigation;

crate::ferro_static_type!(KeyboardNavigation);

crate::ferro_properties! { impl KeyboardNavigation {
    ferro_property!(
        /// Defines the `TabIndex` attached property.
        pub fn tab_index_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<KeyboardNavigation, StyledElement, _>("TabIndex", i32::MAX)
        }
    );

    ferro_property!(
        /// Defines the `TabNavigation` attached property.
        ///
        /// It defines how pressing the Tab key causes focus to be navigated
        /// between the children of the container.
        pub fn tab_navigation_property() -> AttachedProperty<KeyboardNavigationMode> {
            FerroProperty::register_attached::<KeyboardNavigation, InputElement, _>(
                "TabNavigation",
                KeyboardNavigationMode::Continue,
            )
        }
    );

    ferro_property!(
        /// Defines the `TabOnceActiveElement` attached property.
        ///
        /// When focus enters a container which has its `TabNavigation`
        /// attached property set to `Once`, this property defines to which
        /// child the focus should move.
        pub fn tab_once_active_element_property() -> AttachedProperty<Option<Ref<InputElement>>> {
            FerroProperty::register_attached::<KeyboardNavigation, InputElement, _>("TabOnceActiveElement", None)
        }
    );

    ferro_property!(
        /// Defines the `IsTabStop` attached property.
        ///
        /// It determines if the control can receive focus by tab navigation.
        pub fn is_tab_stop_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<KeyboardNavigation, InputElement, _>("IsTabStop", true)
        }
    );
} }

impl KeyboardNavigation {
    /// Gets the `TabIndex` attached property for an element.
    pub fn get_tab_index(element: &InputElement) -> i32 {
        element.get_value(Self::tab_index_property())
    }

    /// Sets the `TabIndex` attached property for an element.
    pub fn set_tab_index(element: &InputElement, value: i32) {
        element.set_value(Self::tab_index_property(), value)
    }

    /// Gets the `TabNavigation` attached property for a container.
    pub fn get_tab_navigation(element: &InputElement) -> KeyboardNavigationMode {
        element.get_value(Self::tab_navigation_property())
    }

    /// Sets the `TabNavigation` attached property for a container.
    pub fn set_tab_navigation(element: &InputElement, value: KeyboardNavigationMode) {
        element.set_value(Self::tab_navigation_property(), value)
    }

    /// Gets the `TabOnceActiveElement` attached property for a container.
    pub fn get_tab_once_active_element(element: &InputElement) -> Option<Ref<InputElement>> {
        element.get_value(Self::tab_once_active_element_property())
    }

    /// Sets the `TabOnceActiveElement` attached property for a container.
    pub fn set_tab_once_active_element(element: &InputElement, value: Option<Ref<InputElement>>) {
        element.set_value(Self::tab_once_active_element_property(), value)
    }

    /// Sets the `IsTabStop` attached property for an element.
    pub fn set_is_tab_stop(element: &InputElement, value: bool) {
        element.set_value(Self::is_tab_stop_property(), value)
    }

    /// Gets the `IsTabStop` attached property for an element.
    pub fn get_is_tab_stop(element: &InputElement) -> bool {
        element.get_value(Self::is_tab_stop_property())
    }
}
