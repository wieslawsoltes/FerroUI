//! `ViewLocator` (ReactiveUI.Binding, `View/ViewLocator.cs`).

use super::ViewLocatorNotFoundException;
use crate::binding::interfaces::IViewLocator;
use ferroui_splat::AppLocator;
use std::rc::Rc;

/// The current [`IViewLocator`] (`ViewLocator`).
pub struct ViewLocator;

impl ViewLocator {
    /// The `dyn IViewLocator` registered in the service locator (`GetCurrent`). It panics with a
    /// [`ViewLocatorNotFoundException`] when none is registered.
    pub fn current() -> Rc<dyn IViewLocator> {
        match AppLocator::get_service::<dyn IViewLocator>() {
            Some(locator) => locator,
            None => std::panic::panic_any(ViewLocatorNotFoundException::new()),
        }
    }
}
