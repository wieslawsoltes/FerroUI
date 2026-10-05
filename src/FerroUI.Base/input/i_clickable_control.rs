use crate::interactivity::RoutedEventArgs;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// A control that can be clicked: what the access key and hot key handling
/// needs from a button-like control.
pub trait IClickableControl {
    /// Raised when the control is clicked. Disposing the returned handle
    /// unsubscribes.
    fn click(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Raises the click of the control.
    fn raise_click(&self);

    /// Gets a value indicating whether this control and all its parents are
    /// enabled.
    fn is_effectively_enabled(&self) -> bool;
}
