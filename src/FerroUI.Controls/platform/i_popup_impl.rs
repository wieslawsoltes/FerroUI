use super::IWindowBaseImpl;
use crate::primitives::popup_positioning::IPopupPositioner;
use std::rc::Rc;

/// Defines a platform-specific popup window implementation.
pub trait IPopupImpl: IWindowBaseImpl {
    /// Gets the positioner of the popup, if the platform provides one.
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>>;

    /// Sets a hint to the window manager whether the popup should have a
    /// shadow.
    fn set_window_manager_add_shadow_hint(&self, enabled: bool);

    /// Allows the popup to take focus.
    fn take_focus(&self);

    /// Sets whether the popup takes part in hit testing.
    fn set_hit_test_visible(&self, is_hit_test_visible: bool);
}
