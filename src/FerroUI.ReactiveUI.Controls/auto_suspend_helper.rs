//! `AutoSuspendHelper` (`AutoSuspendHelper.cs`): automatic suspension and state persistence for
//! FerroUI applications.

use ferroui_controls::application_lifetimes::{ControlledApplicationLifetimeExitEventArgs, IApplicationLifetime};
use ferroui_controls::Design;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::Observable;
use ferroui_reactiveui::RxSuspension;
use ferroui_splat::{AppLocator, IEnableLogger, ILogManager};
use ferroui_system::{IDisposable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Automatic suspension and state persistence for FerroUI applications (`AutoSuspendHelper`).
///
/// Pass the application's lifetime object to [`new`](Self::new), and call
/// [`on_framework_initialization_completed`](Self::on_framework_initialization_completed) from the
/// application's `on_framework_initialization_completed` to signal the launch. The helper connects
/// ReactiveUI's suspension host ([`RxSuspension`]) to the exit of the application and to unhandled
/// errors. It does not persist state in design mode.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the helper is shared (`Rc`), and released with
/// [`dispose`](IDisposable::dispose).
pub struct AutoSuspendHelper {
    this: Weak<AutoSuspendHelper>,
    /// Signals when application state should be persisted (`_shouldPersistState`).
    should_persist_state: Rc<Subject<Rc<dyn IDisposable>>>,
    /// Signals when the application is launching for the first time (`_isLaunchingNew`).
    is_launching_new: Rc<Subject<()>>,
    /// Signals when process state should be invalidated after an unhandled error
    /// (`_shouldInvalidateState`).
    should_invalidate_state: Rc<Subject<()>>,
    /// The subscription to the exit of the controlled lifetime (`_controlledLifetime.Exit`).
    controlled_lifetime_exit: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl IEnableLogger for AutoSuspendHelper {}

impl AutoSuspendHelper {
    /// Connects the suspension host to `lifetime` (`AutoSuspendHelper(IApplicationLifetime)`).
    ///
    /// In design mode, state persistence is disabled. A controlled lifetime has its exit wired to
    /// the persistence of the state. Call it after the application is initialized.
    ///
    /// # Panics
    /// When `lifetime` is `None` (`ArgumentNullException`), and when the lifetime is not a
    /// controlled one, whose exit the helper cannot detect (`NotSupportedException`).
    pub fn new(lifetime: Option<&dyn IApplicationLifetime>) -> Rc<Self> {
        let helper = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            should_persist_state: Subject::new(),
            is_launching_new: Subject::new(),
            should_invalidate_state: Subject::new(),
            controlled_lifetime_exit: RefCell::new(None),
        });

        let host = RxSuspension::suspension_host();
        host.set_is_resuming(Observable::never::<()>());
        host.set_is_launching_new(helper.is_launching_new.as_observable());

        if Design::is_design_mode() {
            helper.log_debug("Design mode detected. AutoSuspendHelper won't persist app state.");
            host.set_should_persist_state(Observable::never::<Rc<dyn IDisposable>>());
        } else if let Some(controlled) = lifetime.and_then(|lifetime| lifetime.as_controlled_application_lifetime()) {
            helper.log_debug("Using IControlledApplicationLifetime events to handle app exit.");
            let weak = helper.this.clone();
            let exit = controlled.exit(Rc::new(move |_args: &ControlledApplicationLifetimeExitEventArgs| {
                if let Some(helper) = weak.upgrade() {
                    helper.on_controlled_application_lifetime_exit();
                }
            }));
            *helper.controlled_lifetime_exit.borrow_mut() = Some(exit);
            host.set_should_persist_state(helper.should_persist_state.as_observable());
        } else if lifetime.is_some() {
            panic!("NotSupportedException: Don't know how to detect app exit event for the application lifetime.");
        } else {
            panic!(
                "ArgumentNullException: lifetime: ApplicationLifetime is null. Ensure you are initializing AutoSuspendHelper \
                 after the application initialization is completed."
            );
        }

        host.set_should_invalidate_state(helper.should_invalidate_state.as_observable());
        helper
    }

    /// Signals that the framework initialization has completed
    /// (`OnFrameworkInitializationCompleted`): notifies the launch of the application.
    pub fn on_framework_initialization_completed(&self) {
        self.is_launching_new.on_next(());
    }

    /// Invalidates the persisted state after an unhandled error (`OnUnhandledException`).
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream subscribes to
    /// `AppDomain.UnhandledException`; Rust has no such event of the process, so the application
    /// calls this method where it handles a failure (a panic hook, the handler of a dispatcher).
    pub fn on_unhandled_exception(&self) {
        self.should_invalidate_state.on_next(());
    }

    /// Handles the exit of a controlled lifetime (`OnControlledApplicationLifetimeExit`): asks the
    /// observers to persist the state.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream blocks the exit until the observers
    /// dispose the token they are handed (`ManualResetEvent.WaitOne`); nothing blocks here (the
    /// threading model has no blocking waits), so an observer persists the state before it returns
    /// and disposes the token then. The helper logs whether that happened.
    fn on_controlled_application_lifetime_exit(&self) {
        self.log_debug("Received IControlledApplicationLifetime exit event.");
        if !self.should_persist_state.has_observers() {
            return;
        }

        let completed = Rc::new(Cell::new(false));
        let signal = completed.clone();
        self.should_persist_state.on_next(Disposable::create(move || signal.set(true)));

        if completed.get() {
            self.log_debug("Completed actions on IControlledApplicationLifetime exit event.");
        } else {
            self.log_debug("The actions on IControlledApplicationLifetime exit event had not completed when it returned.");
        }
    }

    /// Writes a debug message, when a log manager is registered.
    fn log_debug(&self, message: &str) {
        if AppLocator::get_service::<dyn ILogManager>().is_some() {
            self.log().debug(message);
        }
    }
}

impl IDisposable for AutoSuspendHelper {
    /// Releases the helper (`Dispose`): it stops following the exit of the lifetime and completes
    /// nothing more.
    fn dispose(&self) {
        let exit = self.controlled_lifetime_exit.borrow_mut().take();
        if let Some(exit) = exit {
            exit.dispose();
        }

        self.should_persist_state.dispose();
        self.is_launching_new.dispose();
        self.should_invalidate_state.dispose();
    }
}
