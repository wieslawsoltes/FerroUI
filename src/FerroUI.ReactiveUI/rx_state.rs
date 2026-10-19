//! `RxState` (`ReactiveUI.Shared/RxState.cs`).

use crate::{RxSchedulers, UnhandledErrorException};
use ferroui_reactive::concurrency::SchedulerExt;
use ferroui_reactive::Observer;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The handler of the errors nothing else handles (`_defaultExceptionHandler`).
    static DEFAULT_EXCEPTION_HANDLER: RefCell<Option<Rc<dyn IObserver<ObservableError>>>> = const { RefCell::new(None) };
}

/// The message of the exception of the default handler.
const UNHANDLED_ERROR_MESSAGE: &str = "An object implementing IHandleObservableErrors (often a ReactiveCommand or ObservableAsPropertyHelper) has errored, thereby breaking its observable pipeline. To prevent this, ensure the pipeline does not error, or Subscribe to the ThrownExceptions property of the object in question to handle the erroneous case.";

/// The state of ReactiveUI that its objects share (`RxState`): the handler of the errors that no
/// `thrown_exceptions` subscriber handles.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the state is per thread, as the objects that use it are
/// (`docs/porting/reactive.md`, section 5.1).
pub struct RxState;

impl RxState {
    /// The observer that receives the errors of the objects implementing
    /// [`IHandleObservableErrors`](crate::IHandleObservableErrors) when nothing subscribes to
    /// their `thrown_exceptions` (`DefaultExceptionHandler`).
    ///
    /// Unless [`initialize_exception_handler`](Self::initialize_exception_handler) set another
    /// one first, it schedules on [`RxSchedulers::main_thread_scheduler`] a panic with an
    /// [`UnhandledErrorException`] that names the error, where upstream throws that exception.
    pub fn default_exception_handler() -> Rc<dyn IObserver<ObservableError>> {
        if let Some(handler) = DEFAULT_EXCEPTION_HANDLER.with(|slot| slot.borrow().clone()) {
            return handler;
        }
        Self::initialize_default_exception_handler()
    }

    /// Sets the handler of [`default_exception_handler`](Self::default_exception_handler) unless
    /// one was set or read before (`InitializeExceptionHandler`); the builder of the application
    /// calls it.
    pub fn initialize_exception_handler(exception_handler: Rc<dyn IObserver<ObservableError>>) {
        DEFAULT_EXCEPTION_HANDLER.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                *slot = Some(exception_handler);
            }
        });
    }

    /// Forgets the handler, so that the next read or initialization sets it again
    /// (`ResetForTesting`); for tests.
    #[doc(hidden)]
    pub fn reset_for_testing() {
        DEFAULT_EXCEPTION_HANDLER.with(|slot| *slot.borrow_mut() = None);
    }

    fn initialize_default_exception_handler() -> Rc<dyn IObserver<ObservableError>> {
        let handler = Observer::create(|exception: ObservableError| {
            // Upstream breaks into an attached debugger first (`Debugger.Break`); there is no
            // such call here.
            let unhandled = UnhandledErrorException::with_inner(UNHANDLED_ERROR_MESSAGE, &*exception);
            let _ = RxSchedulers::main_thread_scheduler().schedule_action(move || {
                std::panic::panic_any(unhandled);
            });
        });
        DEFAULT_EXCEPTION_HANDLER.with(|slot| slot.borrow_mut().get_or_insert(handler).clone())
    }
}
