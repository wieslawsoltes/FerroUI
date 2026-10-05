use super::{
    ControlledApplicationLifetimeExitEventArgs, ControlledApplicationLifetimeStartupEventArgs, IApplicationLifetime,
    IClassicDesktopStyleApplicationLifetime, IControlledApplicationLifetime, ISetupApplicationLifetime,
    ShutdownRequestedEventArgs,
};
use crate::platform::IPlatformLifetimeEventsImpl;
use crate::{AppBuilder, ShutdownMode, Window, WindowCloseReason};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable};
use ferroui_base::threading::{CancellationTokenSource, Dispatcher};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The application lifetime of classic desktop applications: it tracks the
/// open windows, shuts the application down according to its
/// [`ShutdownMode`] and runs the main loop where the platform has one.
///
/// # Ownership
///
/// The lifetime is a shared handle (`Rc`). Once it has been set up it is
/// kept alive by the window class handlers it registers, by the platform
/// lifetime events it subscribes to and by the application it is the
/// lifetime of (which the service locator holds), so nothing depends on the
/// stack frame of the caller of [`start`](Self::start). Call
/// [`dispose`](Self::dispose) to release those subscriptions.
pub struct ClassicDesktopStyleApplicationLifetime {
    this: Weak<ClassicDesktopStyleApplicationLifetime>,
    exit_code: Cell<i32>,
    cts: RefCell<Option<CancellationTokenSource>>,
    is_shutting_down: Cell<bool>,
    windows: RefCell<Vec<Ref<Window>>>,
    global_events_subscriptions: RefCell<Option<Rc<CompositeDisposable>>>,
    before_init_called: Cell<bool>,
    after_init_called: Cell<bool>,
    platform_lifetime_events_impl: RefCell<Option<Rc<dyn IPlatformLifetimeEventsImpl>>>,
    platform_shutdown_requested: RefCell<Option<Rc<dyn IDisposable>>>,
    startup: HandlerList<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>,
    shutdown_requested: HandlerList<dyn Fn(&ShutdownRequestedEventArgs)>,
    exit: HandlerList<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>,
    args: RefCell<Option<Vec<String>>>,
    shutdown_mode: Cell<ShutdownMode>,
    main_window: RefCell<Option<Ref<Window>>>,
}

impl ClassicDesktopStyleApplicationLifetime {
    /// Creates a lifetime without arguments, with the default shutdown mode
    /// and without a main window.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            exit_code: Cell::new(0),
            cts: RefCell::new(None),
            is_shutting_down: Cell::new(false),
            windows: RefCell::new(Vec::new()),
            global_events_subscriptions: RefCell::new(None),
            before_init_called: Cell::new(false),
            after_init_called: Cell::new(false),
            platform_lifetime_events_impl: RefCell::new(None),
            platform_shutdown_requested: RefCell::new(None),
            startup: HandlerList::new(),
            shutdown_requested: HandlerList::new(),
            exit: HandlerList::new(),
            args: RefCell::new(None),
            shutdown_mode: Cell::new(ShutdownMode::default()),
            main_window: RefCell::new(None),
        })
    }

    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the lifetime is alive")
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&Self) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Raised when the application is starting. Disposing the returned
    /// handle unsubscribes.
    pub fn startup(
        &self,
        handler: impl Fn(&ControlledApplicationLifetimeStartupEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)> = Rc::new(handler);
        self.subscribe(|this| &this.startup, handler)
    }

    /// Raised when an application shutdown is requested; see
    /// [`IClassicDesktopStyleApplicationLifetime::shutdown_requested`].
    /// Disposing the returned handle unsubscribes.
    pub fn shutdown_requested(&self, handler: impl Fn(&ShutdownRequestedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)> = Rc::new(handler);
        self.subscribe(|this| &this.shutdown_requested, handler)
    }

    /// Raised when the application is exiting. Disposing the returned
    /// handle unsubscribes.
    pub fn exit(&self, handler: impl Fn(&ControlledApplicationLifetimeExitEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)> = Rc::new(handler);
        self.subscribe(|this| &this.exit, handler)
    }

    /// The arguments passed to the builder's start method.
    pub fn args(&self) -> Option<Vec<String>> {
        self.args.borrow().clone()
    }

    /// Sets the arguments of the application.
    pub fn set_args(&self, value: Option<Vec<String>>) {
        *self.args.borrow_mut() = value;
    }

    /// The shutdown mode; see
    /// [`IClassicDesktopStyleApplicationLifetime::shutdown_mode`].
    pub fn shutdown_mode(&self) -> ShutdownMode {
        self.shutdown_mode.get()
    }

    /// Sets the shutdown mode.
    pub fn set_shutdown_mode(&self, value: ShutdownMode) {
        self.shutdown_mode.set(value);
    }

    /// The main window of the application.
    pub fn main_window(&self) -> Option<Ref<Window>> {
        self.main_window.borrow().clone()
    }

    /// Sets the main window of the application.
    pub fn set_main_window(&self, value: Option<Ref<Window>>) {
        let old = self.main_window.replace(value);
        drop(old);
    }

    /// The list of all open windows in the application, in the order they
    /// were opened.
    pub fn windows(&self) -> Vec<Ref<Window>> {
        self.windows.borrow().clone()
    }

    fn handle_window_closed(&self, window: &Ref<Window>) {
        if self.is_shutting_down.get() {
            return;
        }

        if self.shutdown_mode() == ShutdownMode::OnLastWindowClose && self.windows.borrow().is_empty() {
            self.try_shutdown(0);
        } else if self.shutdown_mode() == ShutdownMode::OnMainWindowClose
            && self.main_window().as_ref() == Some(window)
        {
            self.try_shutdown(0);
        }
    }

    /// Shuts down the application and sets the exit code that is returned
    /// to the operating system. Windows cannot cancel this shutdown and the
    /// shutdown requested event is not raised.
    pub fn shutdown(&self, exit_code: i32) {
        self.do_shutdown(&ShutdownRequestedEventArgs::new(), true, true, exit_code);
    }

    /// Tries to shut down the application; the shutdown requested event and
    /// the closing event of the windows can cancel the shutdown. Returns
    /// whether the application shuts down.
    ///
    /// # Panics
    /// Panics when the application is already shutting down.
    pub fn try_shutdown(&self, exit_code: i32) -> bool {
        self.do_shutdown(&ShutdownRequestedEventArgs::new(), true, false, exit_code)
    }

    fn subscribe_global_events(&self) {
        debug_assert!(self.global_events_subscriptions.borrow().is_none());

        // The handlers hold the lifetime, as the class handlers of the
        // reference do: it tracks windows until it is disposed.
        let opened = {
            let this = self.this();
            Window::window_opened_event().add_class_handler::<Window>(move |sender, _| {
                let window = sender.to_ref();
                let mut windows = this.windows.borrow_mut();
                if !windows.contains(&window) {
                    windows.push(window);
                }
            })
        };
        let closed = {
            let this = self.this();
            Window::window_closed_event().add_class_handler::<Window>(move |sender, _| {
                let window = sender.to_ref();
                let removed = {
                    let mut windows = this.windows.borrow_mut();
                    windows.iter().position(|w| *w == window).map(|index| windows.remove(index))
                };
                drop(removed);
                this.handle_window_closed(&window);
            })
        };

        *self.global_events_subscriptions.borrow_mut() =
            Some(Rc::new(CompositeDisposable::from_disposables([opened, closed])));
    }

    fn before_init(&self) {
        if self.before_init_called.get() {
            return;
        }

        self.before_init_called.set(true);
        self.subscribe_global_events();
    }

    fn after_init(&self) {
        if self.after_init_called.get() {
            return;
        }

        self.after_init_called.set(true);

        let platform_lifetime_events_impl = FerroLocator::current().get_service::<dyn IPlatformLifetimeEventsImpl>();
        if let Some(platform_lifetime_events_impl) = &platform_lifetime_events_impl {
            let this = self.this();
            let subscription =
                platform_lifetime_events_impl.shutdown_requested(Rc::new(move |e| this.on_shutdown_requested(e)));
            *self.platform_shutdown_requested.borrow_mut() = Some(subscription);
        }
        *self.platform_lifetime_events_impl.borrow_mut() = platform_lifetime_events_impl;
    }

    /// Starts the application: raises the startup event with `args` and
    /// shows the main window, then
    ///
    /// - where the dispatcher of the UI thread supports run loops
    ///   ([`Dispatcher::supports_run_loops`]), runs the main loop until the
    ///   application shuts down and returns the exit code of the shutdown;
    /// - otherwise (platforms whose event loop is owned by the host, such
    ///   as a browser) returns immediately with the exit code as it is at
    ///   that moment: 0, or the code of a shutdown that happened while the
    ///   main window was shown. The application keeps running, driven by
    ///   the host; a later [`shutdown`](Self::shutdown) or
    ///   [`try_shutdown`](Self::try_shutdown) closes the windows, raises the
    ///   exit event (which carries the exit code) and shuts the dispatcher
    ///   down exactly as it does on the platforms with a loop.
    pub fn start(&self, args: &[String]) -> i32 {
        self.start_core(args)
    }

    /// [`start`](Self::start) with the arguments the lifetime was prepared
    /// with ([`args`](Self::args)), for integrating with hosts that manage
    /// the lifetime themselves.
    pub fn start_with_own_args(&self) -> i32 {
        let args = self.args().unwrap_or_default();
        self.start_core(&args)
    }

    fn start_core(&self, args: &[String]) -> i32 {
        // The setup notifications should have come from the application
        // builder. If somehow they did not (e.g., for a manually started
        // lifetime), do it now.
        self.before_init();
        self.after_init();
        let startup_args = ControlledApplicationLifetimeStartupEventArgs::new(args.iter().cloned());
        for (_, handler) in self.startup.snapshot().iter() {
            handler(&startup_args);
        }

        let dispatcher = Dispatcher::ui_thread();
        let runs_main_loop = dispatcher.supports_run_loops();

        // The token source exists exactly while a main loop can be exited:
        // without run loops there is no loop for a shutdown to exit.
        if runs_main_loop {
            *self.cts.borrow_mut() = Some(CancellationTokenSource::new());
        }

        self.show_main_window();

        if runs_main_loop {
            // A shutdown while the main window was shown has taken the
            // source already; the loop must not start then.
            let token = self.cts.borrow().as_ref().map(CancellationTokenSource::token);
            if let Some(token) = token {
                dispatcher.main_loop(&token);
            }
        }

        // The process exit code is the caller's to set (the reference sets
        // the exit code of the environment here).
        self.exit_code.get()
    }

    fn show_main_window(&self) {
        // The window is not held by this frame while it is shown.
        let main_window = self.main_window();
        if let Some(main_window) = &main_window {
            main_window.show();
        }
    }

    /// Stops tracking windows and listening to the platform's shutdown
    /// requests.
    pub fn dispose(&self) {
        let subscriptions = self.global_events_subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }

        let subscription = self.platform_shutdown_requested.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let platform_lifetime_events_impl = self.platform_lifetime_events_impl.borrow_mut().take();
        drop(platform_lifetime_events_impl);
    }

    fn do_shutdown(&self, e: &ShutdownRequestedEventArgs, is_programmatic: bool, force: bool, exit_code: i32) -> bool {
        if !force {
            for (_, handler) in self.shutdown_requested.snapshot().iter() {
                handler(e);
            }

            if e.cancel() {
                return false;
            }

            if self.is_shutting_down.get() {
                panic!("Application is already shutting down.");
            }
        }

        self.exit_code.set(exit_code);
        self.is_shutting_down.set(true);
        let scope = ShutdownScope { lifetime: self, e, shutdown_cancelled: Cell::new(false) };

        // When an OS shutdown request is received, try to close all non-owned windows. Windows can cancel
        // shutdown by setting the cancel flag in the closing event. Owned windows will be shutdown by their
        // owners.
        for w in self.windows() {
            if w.owner().is_none() {
                let ignore_cancel = force
                    || (self.shutdown_mode() == ShutdownMode::OnMainWindowClose
                        && self.main_window().as_ref() != Some(&w));
                let reason = if e.is_os_shutdown() {
                    WindowCloseReason::OSShutdown
                } else {
                    WindowCloseReason::ApplicationShutdown
                };
                w.close_core(reason, is_programmatic, ignore_cancel);
            }
        }

        if !force && !self.windows.borrow().is_empty() {
            e.set_cancel(true);
            scope.shutdown_cancelled.set(true);
            return false;
        }

        let args = ControlledApplicationLifetimeExitEventArgs::new(exit_code);
        for (_, handler) in self.exit.snapshot().iter() {
            handler(&args);
        }
        self.exit_code.set(args.application_exit_code());

        true
    }

    fn on_shutdown_requested(&self, e: &ShutdownRequestedEventArgs) {
        self.do_shutdown(e, false, false, 0);
    }
}

/// The `finally` block of a shutdown: it runs however the shutdown ends.
struct ShutdownScope<'a> {
    lifetime: &'a ClassicDesktopStyleApplicationLifetime,
    e: &'a ShutdownRequestedEventArgs,
    shutdown_cancelled: Cell<bool>,
}

impl Drop for ShutdownScope<'_> {
    fn drop(&mut self) {
        self.lifetime.is_shutting_down.set(false);

        if !self.shutdown_cancelled.get() {
            let cts = self.lifetime.cts.borrow_mut().take();
            self.e.set_will_exit_main_loop(cts.is_some());

            if let Some(cts) = cts {
                cts.cancel();
            }
            Dispatcher::ui_thread().invoke_shutdown();
        }
    }
}

impl IApplicationLifetime for ClassicDesktopStyleApplicationLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_controlled_application_lifetime(&self) -> Option<&dyn IControlledApplicationLifetime> {
        Some(self)
    }

    fn as_setup_application_lifetime(&self) -> Option<&dyn ISetupApplicationLifetime> {
        Some(self)
    }

    fn as_classic_desktop_style_application_lifetime(&self) -> Option<&dyn IClassicDesktopStyleApplicationLifetime> {
        Some(self)
    }
}

impl IControlledApplicationLifetime for ClassicDesktopStyleApplicationLifetime {
    fn startup(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.startup, handler)
    }

    fn exit(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.exit, handler)
    }

    fn shutdown(&self, exit_code: i32) {
        ClassicDesktopStyleApplicationLifetime::shutdown(self, exit_code)
    }
}

impl IClassicDesktopStyleApplicationLifetime for ClassicDesktopStyleApplicationLifetime {
    fn try_shutdown(&self, exit_code: i32) -> bool {
        ClassicDesktopStyleApplicationLifetime::try_shutdown(self, exit_code)
    }

    fn args(&self) -> Option<Vec<String>> {
        ClassicDesktopStyleApplicationLifetime::args(self)
    }

    fn shutdown_mode(&self) -> ShutdownMode {
        ClassicDesktopStyleApplicationLifetime::shutdown_mode(self)
    }

    fn set_shutdown_mode(&self, value: ShutdownMode) {
        ClassicDesktopStyleApplicationLifetime::set_shutdown_mode(self, value)
    }

    fn main_window(&self) -> Option<Ref<Window>> {
        ClassicDesktopStyleApplicationLifetime::main_window(self)
    }

    fn set_main_window(&self, value: Option<Ref<Window>>) {
        ClassicDesktopStyleApplicationLifetime::set_main_window(self, value)
    }

    fn windows(&self) -> Vec<Ref<Window>> {
        ClassicDesktopStyleApplicationLifetime::windows(self)
    }

    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.shutdown_requested, handler)
    }
}

impl ISetupApplicationLifetime for ClassicDesktopStyleApplicationLifetime {
    fn before_app_init(&self) {
        self.before_init()
    }

    fn after_app_init(&self) {
        self.after_init()
    }
}

impl IDisposable for ClassicDesktopStyleApplicationLifetime {
    fn dispose(&self) {
        ClassicDesktopStyleApplicationLifetime::dispose(self)
    }
}

fn create_lifetime(
    args: &[String],
    lifetime_builder: Option<&dyn Fn(&Rc<ClassicDesktopStyleApplicationLifetime>)>,
) -> Rc<ClassicDesktopStyleApplicationLifetime> {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_args(Some(args.to_vec()));
    if let Some(lifetime_builder) = lifetime_builder {
        lifetime_builder(&lifetime);
    }
    lifetime
}

/// The classic desktop lifetime related methods of the application builder.
impl AppBuilder {
    /// Sets the application up with a classic desktop lifetime, but doesn't
    /// show the main window and doesn't run the application main loop.
    ///
    /// `args` are the startup arguments.
    pub fn setup_with_classic_desktop_lifetime(&self, args: &[String]) -> AppBuilder {
        let lifetime = create_lifetime(args, None);
        self.setup_with_lifetime(lifetime)
    }

    /// [`setup_with_classic_desktop_lifetime`](Self::setup_with_classic_desktop_lifetime)
    /// with a `lifetime_builder` that modifies the lifetime before the
    /// application is set up. The builder receives the handle of the
    /// lifetime so that it can keep it.
    pub fn setup_with_classic_desktop_lifetime_with(
        &self,
        args: &[String],
        lifetime_builder: impl Fn(&Rc<ClassicDesktopStyleApplicationLifetime>),
    ) -> AppBuilder {
        let lifetime = create_lifetime(args, Some(&lifetime_builder));
        self.setup_with_lifetime(lifetime)
    }

    /// Starts the application with a classic desktop lifetime: sets the
    /// application up, shows the main window and runs the application main
    /// loop where the platform has one.
    ///
    /// Returns what [`ClassicDesktopStyleApplicationLifetime::start`]
    /// returns: the exit code once the main loop has ended or, on platforms
    /// whose dispatcher does not support run loops, immediately after the
    /// start-up, with the application left running (the application, the
    /// lifetime and the windows are owned by the service locator, the class
    /// handlers of the lifetime and the platform windows, not by the
    /// caller).
    pub fn start_with_classic_desktop_lifetime(&self, args: &[String]) -> i32 {
        let lifetime = create_lifetime(args, None);
        self.setup_with_lifetime(lifetime.clone());
        lifetime.start(args)
    }

    /// [`start_with_classic_desktop_lifetime`](Self::start_with_classic_desktop_lifetime)
    /// with a `lifetime_builder` that modifies the lifetime before the
    /// application is started.
    pub fn start_with_classic_desktop_lifetime_with(
        &self,
        args: &[String],
        lifetime_builder: impl Fn(&Rc<ClassicDesktopStyleApplicationLifetime>),
    ) -> i32 {
        let lifetime = create_lifetime(args, Some(&lifetime_builder));
        self.setup_with_lifetime(lifetime.clone());
        lifetime.start(args)
    }

    /// [`start_with_classic_desktop_lifetime`](Self::start_with_classic_desktop_lifetime)
    /// with the shutdown mode of the lifetime.
    pub fn start_with_classic_desktop_lifetime_and_shutdown_mode(
        &self,
        args: &[String],
        shutdown_mode: ShutdownMode,
    ) -> i32 {
        let lifetime = create_lifetime(args, Some(&|l| l.set_shutdown_mode(shutdown_mode)));
        self.setup_with_lifetime(lifetime.clone());
        lifetime.start(args)
    }
}
