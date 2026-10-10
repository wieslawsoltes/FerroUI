//! The back button of an activity, as a service: who raises the request,
//! and the arguments of it.

use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

pub trait IActivityNavigationService {
    /// Raised when the user asks to go back.
    ///
    /// Disposing the returned value removes the handler.
    fn back_requested(&self, handler: Rc<dyn Fn(&AndroidBackRequestedEventArgs)>) -> Rc<dyn IDisposable>;
}

#[derive(Default)]
pub struct AndroidBackRequestedEventArgs {
    handled: Cell<bool>,
}

impl AndroidBackRequestedEventArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
