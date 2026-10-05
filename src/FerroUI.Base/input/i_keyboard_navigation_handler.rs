use super::{InputElement, KeyDeviceType, KeyModifiers, NavigationDirection};
use crate::Ref;

/// Defines the interface for classes that handle keyboard navigation for a
/// window.
pub trait IKeyboardNavigationHandler {
    /// Sets the owner of the keyboard navigation handler.
    ///
    /// This method can only be called once, typically by the owner itself
    /// on creation.
    fn set_owner(&self, owner: &Ref<InputElement>);

    /// Moves the focus in the specified direction.
    ///
    /// `element` is the current element, `key_modifiers` any key modifiers
    /// active at the time of focus and `device_type` the optional key
    /// device type that caused the move.
    fn move_(
        &self,
        element: Option<&Ref<InputElement>>,
        direction: NavigationDirection,
        key_modifiers: KeyModifiers,
        device_type: Option<KeyDeviceType>,
    ) -> bool;
}
