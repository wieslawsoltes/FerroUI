use super::{Cursor, KeyModifiers, NavigationMethod};
use std::rc::Rc;

/// Defines input-related functionality for a control.
///
/// The only implementor is `InputElement`: object references to input
/// elements are `Ref<InputElement>`. The trait documents the contract; the
/// routed event members of the contract (`add_handler`, `remove_handler`,
/// `raise_event`) are inherited by every input element from `Interactive`.
pub trait IInputElement {
    /// Whether the control can receive keyboard focus.
    fn focusable(&self) -> bool;

    /// Whether the control is enabled for user interaction.
    fn is_enabled(&self) -> bool;

    /// The associated mouse cursor.
    fn cursor(&self) -> Option<Rc<Cursor>>;

    /// Whether this control and all its parents are enabled.
    ///
    /// `is_enabled` is used to toggle the enabled state for individual
    /// controls. This property takes into account the enabled state of this
    /// control and its parent controls.
    fn is_effectively_enabled(&self) -> bool;

    /// Whether this control and all its parents are visible.
    fn is_effectively_visible(&self) -> bool;

    /// Whether keyboard focus is anywhere within the element or its visual
    /// tree child elements.
    fn is_keyboard_focus_within(&self) -> bool;

    /// Whether the control is focused.
    fn is_focused(&self) -> bool;

    /// Whether the control is considered for hit testing.
    fn is_hit_test_visible(&self) -> bool;

    /// Whether the pointer is currently over the control.
    fn is_pointer_over(&self) -> bool;

    /// Focuses the control. Returns whether the control was focused.
    fn focus(&self, method: NavigationMethod, key_modifiers: KeyModifiers) -> bool;
}
