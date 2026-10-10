//! The contract of a dispatcher implementation of the X11 platform (the
//! port of `IX11PlatformDispatcher.cs`).

use super::X11EventDispatcher;
use ferroui_base::threading::IDispatcherImpl;
use std::rc::Rc;

/// A dispatcher implementation that reads the events of the connection.
pub trait IX11PlatformDispatcher: IDispatcherImpl {
    /// What takes the events from the connection and hands them out.
    fn event_dispatcher(&self) -> &Rc<X11EventDispatcher>;
}
