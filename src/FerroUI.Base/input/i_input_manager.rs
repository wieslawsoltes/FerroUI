use super::raw::IRawInputEventArgs;
use crate::reactive::IObservable;
use std::rc::Rc;

/// Receives input from the windowing subsystem and dispatches it to
/// interested parties for processing.
pub trait IInputManager {
    /// An observable that notifies on each input event received before
    /// `process`.
    fn pre_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>>;

    /// An observable that notifies on each input event received.
    fn process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>>;

    /// An observable that notifies on each input event received after
    /// `process`.
    fn post_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>>;

    /// Processes a raw input event.
    fn process_input(&self, e: Rc<dyn IRawInputEventArgs>);
}
