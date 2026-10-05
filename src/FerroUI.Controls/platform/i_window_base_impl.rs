use super::ITopLevelImpl;
use ferroui_base::{PixelPoint, Size};
use std::rc::Rc;

/// Defines a platform-specific window base implementation: what windows and
/// popups have in common.
pub trait IWindowBaseImpl: ITopLevelImpl {
    /// Gets the total size of the toplevel, excluding shadows.
    fn frame_size(&self) -> Option<Size>;

    /// Shows the window.
    ///
    /// `activate` is whether to activate the shown window; `is_dialog` is
    /// whether the window is being shown as a dialog.
    fn show(&self, activate: bool, is_dialog: bool);

    /// Hides the window.
    fn hide(&self);

    /// Gets the position of the window in device pixels.
    fn position(&self) -> PixelPoint;

    /// Gets the method called when the window's position changes.
    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>>;

    /// Sets a method called when the window's position changes.
    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>);

    /// Activates the window.
    fn activate(&self);

    /// Gets the method called when the window is deactivated (loses
    /// focus).
    fn deactivated(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when the window is deactivated (loses focus).
    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>);

    /// Gets the method called when the window is activated (receives
    /// focus).
    fn activated(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when the window is activated (receives focus).
    fn set_activated(&self, value: Option<Rc<dyn Fn()>>);

    /// Gets a maximum client size hint for an auto-sizing window, in
    /// device-independent pixels.
    fn max_auto_size_hint(&self) -> Size;

    /// Sets whether this window appears on top of all other windows.
    fn set_topmost(&self, value: bool);
}
