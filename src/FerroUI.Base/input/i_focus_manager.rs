use super::{FindNextElementOptions, InputElement, KeyModifiers, NavigationDirection, NavigationMethod};
use crate::Ref;
use std::any::Any;

/// Manages focus for the application.
pub trait IFocusManager {
    /// Gets the currently focused element.
    fn get_focused_element(&self) -> Option<Ref<InputElement>>;

    /// Focuses a control.
    ///
    /// `element` is the control to focus; `None` clears the focus. `method`
    /// is the method by which focus was changed and `key_modifiers` any key
    /// modifiers active at the time of focus. Returns true if the focus
    /// moved to a control.
    fn focus(&self, element: Option<&Ref<InputElement>>, method: NavigationMethod, key_modifiers: KeyModifiers)
        -> bool;

    /// Attempts to change focus from the element with focus to the next
    /// focusable element in the specified direction.
    ///
    /// `direction` must be one of next, previous, left, right, up and down.
    /// `options` help identify the next element to receive focus. Returns
    /// true if focus moved.
    fn try_move_focus(&self, direction: NavigationDirection, options: Option<&FindNextElementOptions>) -> bool;

    /// Retrieves the first element that can receive focus.
    fn find_first_focusable_element(&self) -> Option<Ref<InputElement>>;

    /// Retrieves the last element that can receive focus.
    fn find_last_focusable_element(&self) -> Option<Ref<InputElement>>;

    /// Retrieves the element that should receive focus based on the
    /// specified navigation direction.
    ///
    /// `direction` must be one of next, previous, left, right, up and down.
    /// `options` help identify the next element to receive focus.
    fn find_next_element(
        &self,
        direction: NavigationDirection,
        options: Option<&FindNextElementOptions>,
    ) -> Option<Ref<InputElement>>;

    /// The focus manager as [`Any`], for downcasting to the concrete type.
    fn as_any(&self) -> &dyn Any;
}
