use super::raw::IRawInputEventArgs;
use super::IInputManager;
use crate::reactive::{IObservable, IObserver, LightweightSubject};
use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

type RawSubject = LightweightSubject<Rc<dyn IRawInputEventArgs>>;

/// Receives input from the windowing subsystem and dispatches it to
/// interested parties for processing.
pub struct InputManager {
    pre_process: RawSubject,
    process: RawSubject,
    post_process: RawSubject,
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    /// Creates an input manager.
    pub fn new() -> Self {
        Self {
            pre_process: LightweightSubject::new(),
            process: LightweightSubject::new(),
            post_process: LightweightSubject::new(),
        }
    }

    /// The input manager registered with the locator.
    pub fn instance() -> Option<Rc<dyn IInputManager>> {
        FerroLocator::current().get_service::<dyn IInputManager>()
    }

    /// Completes the observables of the input manager.
    pub fn dispose(&self) {
        self.pre_process.on_completed();
        self.process.on_completed();
        self.post_process.on_completed();
    }
}

impl IInputManager for InputManager {
    fn pre_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        Rc::new(self.pre_process.clone())
    }

    fn process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        Rc::new(self.process.clone())
    }

    fn post_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        Rc::new(self.post_process.clone())
    }

    fn process_input(&self, e: Rc<dyn IRawInputEventArgs>) {
        self.pre_process.on_next(e.clone());
        e.device().process_raw_event(&*e);
        self.process.on_next(e.clone());
        self.post_process.on_next(e);
    }
}
