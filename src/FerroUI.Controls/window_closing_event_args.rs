use std::cell::Cell;
use std::rc::Rc;

/// Specifies the reason that a window was closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowCloseReason {
    /// The cause of the closure was not provided by the underlying platform.
    #[default]
    Undefined = 0,

    /// The window itself was requested to close.
    WindowClosing = 1,

    /// The window is closing due to a parent/owner window closing.
    OwnerWindowClosing = 2,

    /// The window is closing due to the application shutting down.
    ApplicationShutdown = 3,

    /// The window is closing due to the operating system shutting down.
    OSShutdown = 4,
}

/// Provides data for the window closing event.
///
/// The args are a reference object: a copy shares the cancel flag with the
/// args it was made from, so that a handler given a copy cancels the
/// closing, and copies compare by identity.
#[derive(Clone, Debug)]
pub struct WindowClosingEventArgs {
    cancel: Rc<Cell<bool>>,
    close_reason: WindowCloseReason,
    is_programmatic: bool,
}

impl PartialEq for WindowClosingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cancel, &other.cancel)
    }
}

impl WindowClosingEventArgs {
    /// Creates the event args. Only the windowing classes raise the event.
    pub fn new(reason: WindowCloseReason, is_programmatic: bool) -> Self {
        Self { cancel: Rc::new(Cell::new(false)), close_reason: reason, is_programmatic }
    }

    /// Whether the event should be canceled.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets whether the event should be canceled.
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }

    /// Gets a value that indicates why the window is being closed.
    pub fn close_reason(&self) -> WindowCloseReason {
        self.close_reason
    }

    /// Gets a value indicating whether the window is being closed
    /// programmatically.
    pub fn is_programmatic(&self) -> bool {
        self.is_programmatic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_reason_and_can_be_canceled() {
        let e = WindowClosingEventArgs::new(WindowCloseReason::OwnerWindowClosing, true);
        assert_eq!(e.close_reason(), WindowCloseReason::OwnerWindowClosing);
        assert!(e.is_programmatic());
        assert!(!e.cancel());
        e.set_cancel(true);
        assert!(e.cancel());
    }

    #[test]
    fn a_copy_shares_the_cancel_flag_and_the_identity() {
        let e = WindowClosingEventArgs::new(WindowCloseReason::WindowClosing, false);
        let copy = e.clone();
        copy.set_cancel(true);
        assert!(e.cancel());
        assert!(e == copy);
        assert!(e != WindowClosingEventArgs::new(WindowCloseReason::WindowClosing, false));

        let boxed: ferroui_base::BoxedValue = Rc::new(e.clone());
        let unboxed = boxed.downcast_ref::<WindowClosingEventArgs>().expect("the args");
        unboxed.set_cancel(false);
        assert!(!e.cancel());
    }

    #[test]
    fn close_reason_values_match_the_reference_values() {
        assert_eq!(WindowCloseReason::Undefined as i32, 0);
        assert_eq!(WindowCloseReason::WindowClosing as i32, 1);
        assert_eq!(WindowCloseReason::OwnerWindowClosing as i32, 2);
        assert_eq!(WindowCloseReason::ApplicationShutdown as i32, 3);
        assert_eq!(WindowCloseReason::OSShutdown as i32, 4);
    }
}
