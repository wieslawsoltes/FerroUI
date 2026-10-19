//! `DummySuspensionDriver` (`ReactiveUI.Shared/Suspension/DummySuspensionDriver.cs`).

use crate::{ISuspensionDriver, ISuspensionDriverOf};
use ferroui_reactive::Observable;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// A driver that persists nothing (`DummySuspensionDriver`), for tests and for applications that
/// do not need their state saved: loading gives no state, and saving and invalidating complete at
/// once.
#[derive(Clone, Copy, Debug, Default)]
pub struct DummySuspensionDriver;

impl DummySuspensionDriver {
    /// The driver.
    pub fn new() -> Self {
        Self
    }
}

impl ISuspensionDriver for DummySuspensionDriver {
    fn load_state(&self) -> Rc<dyn IObservable<Option<Rc<dyn Any>>>> {
        Observable::return_(None)
    }

    fn save_state(&self, _state: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<()>> {
        Observable::return_(())
    }

    fn invalidate_state(&self) -> Rc<dyn IObservable<()>> {
        Observable::return_(())
    }
}

impl<T: 'static> ISuspensionDriverOf<T> for DummySuspensionDriver {
    fn load_state_of(&self) -> Rc<dyn IObservable<Option<Rc<T>>>> {
        Observable::return_(None)
    }

    fn save_state_of(&self, _state: Option<Rc<T>>) -> Rc<dyn IObservable<()>> {
        Observable::return_(())
    }
}
