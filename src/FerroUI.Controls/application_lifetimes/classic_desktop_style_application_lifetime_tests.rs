//! Port of the reference `DesktopStyleApplicationLifetimeTests`.

use super::{ClassicDesktopStyleApplicationLifetime, ISetupApplicationLifetime, ShutdownRequestedEventArgs};
use crate::platform::{IPlatformLifetimeEventsImpl, IScreenImpl, ITopLevelImpl};
use crate::testing::{
    mock_screen, MockImplKind, MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices,
    UnitTestApplication,
};
use crate::{AppBuilder, Application, ApplicationImpl, ApplicationImplExt, ShutdownMode, Window};
use ferroui_base::platform::ManagedDispatcherImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{
    Dispatcher, DispatcherImplEvent, DispatcherPriority, IDispatcherImpl, IDispatcherSignal, UnitTestDispatcherScope,
};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, PixelRect, PixelSize, Ref,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// The platform lifetime events of a test: the shutdown request is raised
/// by hand.
#[derive(Default)]
struct TestPlatformLifetimeEvents {
    shutdown_requested: Rc<HandlerList<dyn Fn(&ShutdownRequestedEventArgs)>>,
}

impl TestPlatformLifetimeEvents {
    fn raise_shutdown_requested(&self, e: &ShutdownRequestedEventArgs) {
        for (_, handler) in self.shutdown_requested.snapshot().iter() {
            handler(e);
        }
    }
}

impl IPlatformLifetimeEventsImpl for TestPlatformLifetimeEvents {
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.shutdown_requested.add(handler);
        let handlers = self.shutdown_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

fn setup(lifetime: &ClassicDesktopStyleApplicationLifetime) {
    let setup_lifetime: &dyn ISetupApplicationLifetime = lifetime;
    setup_lifetime.before_app_init();
    setup_lifetime.after_app_init();
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn flag() -> Rc<Cell<bool>> {
    Rc::new(Cell::new(false))
}

fn track_exit(lifetime: &ClassicDesktopStyleApplicationLifetime) -> Rc<Cell<bool>> {
    let has_exit = flag();
    let h = has_exit.clone();
    lifetime.exit(move |_| h.set(true));
    has_exit
}

fn shown_window() -> Ref<Window> {
    let window = Window::new();
    window.show();
    window
}

fn is_only_window(lifetime: &ClassicDesktopStyleApplicationLifetime, window: &Ref<Window>) -> bool {
    let windows = lifetime.windows();
    windows.len() == 1 && windows[0] == *window
}

#[test]
fn should_set_exit_code_after_shutdown() {
    let _app = UnitTestApplication::start(TestServices::new());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let l = lifetime.clone();
    Dispatcher::ui_thread().post_local(move || l.shutdown(1337), DispatcherPriority::DEFAULT);
    let exit_code = lifetime.start(&[]);

    assert_eq!(1337, exit_code);
    lifetime.dispose();
}

#[test]
fn should_close_all_remaining_open_windows_after_explicit_exit_call() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let windows = vec![Window::new(), Window::new(), Window::new(), Window::new()];

    for window in &windows {
        window.show();
    }
    assert_eq!(4, lifetime.windows().len());
    lifetime.shutdown(0);

    assert!(lifetime.windows().is_empty());
    lifetime.dispose();
}

#[test]
fn should_only_exit_on_explicit_exit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnExplicitShutdown);
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let window_a = shown_window();
    let window_b = shown_window();

    window_a.close();

    assert!(!has_exit.get());

    window_b.close();

    assert!(!has_exit.get());

    lifetime.shutdown(0);

    assert!(has_exit.get());
    lifetime.dispose();
}

#[test]
fn should_exit_after_main_window_closed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnMainWindowClose);
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let main_window = shown_window();

    lifetime.set_main_window(Some(main_window.clone()));

    let _window = shown_window();

    main_window.close();

    assert!(has_exit.get());
    lifetime.dispose();
}

/// The common part of the two tests of the secondary window that cancels
/// its closing; `shutdown` starts the shutdown.
fn secondary_window_cancellation_is_overridden(
    shutdown: impl FnOnce(&ClassicDesktopStyleApplicationLifetime, &Ref<Window>),
) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnMainWindowClose);
    setup(&lifetime);

    let secondary_window_closing_executed = flag();
    let secondary_window_closed_executed = flag();

    let has_exit = track_exit(&lifetime);

    let main_window = shown_window();

    lifetime.set_main_window(Some(main_window.clone()));

    let window = Window::new();
    let executed = secondary_window_closing_executed.clone();
    window.closing(move |args| {
        executed.set(true);
        args.set_cancel(true);
    });
    let executed = secondary_window_closed_executed.clone();
    window.closed(move || executed.set(true));
    window.show();

    shutdown(&lifetime, &main_window);

    assert!(secondary_window_closing_executed.get());
    assert!(secondary_window_closed_executed.get());
    assert!(has_exit.get());
    lifetime.dispose();
}

#[test]
fn on_main_window_close_overrides_secondary_window_cancellation() {
    secondary_window_cancellation_is_overridden(|_, main_window| main_window.close());
}

#[test]
fn on_main_window_close_overrides_secondary_window_cancellation_from_try_shutdown() {
    secondary_window_cancellation_is_overridden(|lifetime, _| {
        lifetime.try_shutdown(0);
    });
}

#[test]
fn should_exit_after_last_window_closed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnLastWindowClose);
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let window_a = shown_window();
    let window_b = shown_window();

    window_a.close();

    assert!(!has_exit.get());

    window_b.close();

    assert!(has_exit.get());
    lifetime.dispose();
}

#[test]
fn show_should_add_window_to_open_windows() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let window = Window::new();

    window.show();

    assert!(is_only_window(&lifetime, &window));
    lifetime.dispose();
}

#[test]
fn window_should_be_added_to_open_windows_only_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let window = Window::new();

    window.show();
    window.show();
    window.set_is_visible(true);

    assert!(is_only_window(&lifetime, &window));

    window.close();
    lifetime.dispose();
}

#[test]
fn close_should_remove_window_from_open_windows() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let window = Window::new();

    window.show();
    assert_eq!(1, lifetime.windows().len());
    window.close();

    assert!(lifetime.windows().is_empty());
    lifetime.dispose();
}

#[test]
fn impl_closing_should_remove_window_from_open_windows() {
    let window_impl = MockWindowImpl::bare(MockImplKind::Window);
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);

    let screen1 = mock_screen(
        1.75,
        PixelRect::from_size(PixelSize::new(1920, 1080)),
        PixelRect::from_size(PixelSize::new(1920, 966)),
        true,
    );
    let screens = MockScreenImpl::new(vec![screen1]);
    screens.set_resolves_screens(true);
    window_impl.setup_feature::<dyn IScreenImpl>(screens);

    let created = window_impl.clone();
    let services = TestServices::styled_window()
        .with_windowing_platform(MockWindowingPlatform::with_window_impl(move || created.clone()));

    let _app = UnitTestApplication::start(services);
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let window = Window::new();

    window.show();
    assert_eq!(1, lifetime.windows().len());
    ITopLevelImpl::closed(&*window_impl).unwrap()();

    assert!(lifetime.windows().is_empty());
    lifetime.dispose();
}

#[test]
fn should_allow_canceling_shutdown_via_shutdown_requested_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    let lifetime_events = Rc::new(TestPlatformLifetimeEvents::default());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformLifetimeEventsImpl>()
        .to_constant(lifetime_events.clone() as Rc<dyn IPlatformLifetimeEventsImpl>);

    // Force exit immediately
    let dispatcher = Dispatcher::ui_thread();
    let d = dispatcher.clone();
    dispatcher.post_local(move || d.exit_all_frames(), DispatcherPriority::DEFAULT);
    lifetime.start(&[]);

    let window = Window::new();
    let raised = counter();

    window.show();

    let r = raised.clone();
    lifetime.shutdown_requested(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });

    lifetime_events.raise_shutdown_requested(&ShutdownRequestedEventArgs::new());

    assert_eq!(1, raised.get());
    assert!(is_only_window(&lifetime, &window));
    lifetime.dispose();
}

#[test]
fn main_window_closed_shutdown_should_be_cancellable() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnMainWindowClose);
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let main_window = shown_window();

    lifetime.set_main_window(Some(main_window.clone()));

    let _window = shown_window();

    let raised = counter();

    let r = raised.clone();
    lifetime.shutdown_requested(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });

    main_window.close();

    assert_eq!(1, raised.get());
    assert!(!has_exit.get());
    lifetime.dispose();
}

#[test]
fn last_window_closed_shutdown_should_be_cancellable() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnLastWindowClose);
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let window_a = shown_window();
    let window_b = shown_window();

    let raised = counter();

    let r = raised.clone();
    lifetime.shutdown_requested(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });

    window_a.close();

    assert!(!has_exit.get());

    window_b.close();

    assert_eq!(1, raised.get());
    assert!(!has_exit.get());
    lifetime.dispose();
}

#[test]
fn try_shutdown_cancellable_by_preventing_window_close() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    lifetime.exit(|_| panic!("lifetime.Exit was called."));
    let shutdown_started = Dispatcher::ui_thread()
        .shutdown_started(|_| panic!("Dispatcher.UIThread.ShutdownStarted was called."));

    let window_a = shown_window();
    let _window_b = shown_window();

    let raised = counter();

    let r = raised.clone();
    window_a.closing(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });

    lifetime.try_shutdown(0);

    assert_eq!(1, raised.get());

    shutdown_started.dispose();
    lifetime.dispose();
}

#[test]
fn shutdown_not_cancellable_by_preventing_window_close() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let closing_raised = counter();
    let closed_raised = counter();

    let has_exit = track_exit(&lifetime);

    let window_a = shown_window();
    let _window_b = shown_window();

    let r = closing_raised.clone();
    window_a.closing(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });
    let r = closed_raised.clone();
    window_a.closed(move || r.set(r.get() + 1));

    lifetime.shutdown(0);

    assert_eq!(1, closing_raised.get());
    assert_eq!(1, closed_raised.get());
    assert!(has_exit.get());
    lifetime.dispose();
}

#[test]
fn shutdown_doesnt_raise_shutdown_requested() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);

    let has_exit = track_exit(&lifetime);

    let raised = counter();

    let r = raised.clone();
    lifetime.shutdown_requested(move |_| r.set(r.get() + 1));

    lifetime.shutdown(0);

    assert_eq!(0, raised.get());
    assert!(has_exit.get());
    lifetime.dispose();
}

// --- the application builder ------------------------------------------------

/// The application of the builder tests: a unit test application that the
/// builder can register the services of a second time.
#[repr(C)]
struct SetupTestApplication {
    base: UnitTestApplication,
    services_registered: Cell<bool>,
    on_framework_initialization_completed: Option<Rc<dyn Fn()>>,
}

ferro_class!(SetupTestApplication: UnitTestApplication);
ferro_impl_classes!(SetupTestApplication: FerroObjectImpl);

impl ApplicationImpl for SetupTestApplication {
    fn register_services(this: &Self) {
        if this.services_registered.get() {
            return;
        }

        this.services_registered.set(true);
        Self::parent_register_services(this);
    }

    fn on_framework_initialization_completed(this: &Self) {
        Self::parent_on_framework_initialization_completed(this);
        if let Some(callback) = &this.on_framework_initialization_completed {
            callback();
        }
    }
}

/// Isolates the dispatcher, the service locator and the setup flag of the
/// builder (the tests above get this from the unit test application).
struct BuilderScope {
    locator: Rc<dyn IDisposable>,
    _dispatcher: UnitTestDispatcherScope,
}

fn builder_scope() -> BuilderScope {
    let dispatcher = Dispatcher::unit_test_scope();
    BuilderScope { locator: FerroLocator::enter_scope(), _dispatcher: dispatcher }
}

impl Drop for BuilderScope {
    fn drop(&mut self) {
        self.locator.dispose();
        AppBuilder::reset_setup_for_unit_tests();
    }
}

fn create_app_builder(
    platform_lifetime_events: Option<Rc<dyn IPlatformLifetimeEventsImpl>>,
    on_framework_initialization_completed: Option<Rc<dyn Fn()>>,
) -> AppBuilder {
    AppBuilder::reset_setup_for_unit_tests();

    AppBuilder::configure_with(move || {
        instantiate(SetupTestApplication {
            base: UnitTestApplication::construct(TestServices::styled_window()),
            services_registered: Cell::new(false),
            on_framework_initialization_completed: on_framework_initialization_completed.clone(),
        })
    })
    .use_runtime_platform_subsystem(|| {}, "")
    .use_rendering_subsystem(|| {}, "")
    .use_text_shaping_subsystem(|| {}, "")
    .use_windowing_subsystem(
        move || {
            if let Some(platform_lifetime_events) = &platform_lifetime_events {
                FerroLocator::current_mutable()
                    .bind::<dyn IPlatformLifetimeEventsImpl>()
                    .to_constant(platform_lifetime_events.clone());
            }
        },
        "",
    )
}

type LifetimeSlot = Rc<std::cell::RefCell<Option<Rc<ClassicDesktopStyleApplicationLifetime>>>>;

#[test]
fn setup_with_classic_desktop_lifetime_should_subscribe_to_platform_shutdown_requested() {
    let _scope = builder_scope();
    let lifetime_events = Rc::new(TestPlatformLifetimeEvents::default());
    let lifetime: LifetimeSlot = Default::default();

    let slot = lifetime.clone();
    create_app_builder(Some(lifetime_events.clone()), None)
        .setup_with_classic_desktop_lifetime_with(&[], move |l| *slot.borrow_mut() = Some(l.clone()));

    let lifetime = lifetime.borrow().clone().expect("the lifetime builder was called");

    let window = Window::new();
    window.show();

    let raised = counter();

    let r = raised.clone();
    lifetime.shutdown_requested(move |e| {
        e.set_cancel(true);
        r.set(r.get() + 1);
    });

    lifetime_events.raise_shutdown_requested(&ShutdownRequestedEventArgs::new());

    assert_eq!(1, raised.get());
    assert!(is_only_window(&lifetime, &window));
    lifetime.dispose();
}

#[test]
fn setup_with_classic_desktop_lifetime_should_not_raise_startup() {
    let _scope = builder_scope();
    let framework_init_called = flag();
    let lifetime_builder_called = flag();
    let startup_raised = flag();

    let called = framework_init_called.clone();
    let builder_called = lifetime_builder_called.clone();
    let raised = startup_raised.clone();
    let args = ["foo".to_string(), "bar".to_string()];
    create_app_builder(None, Some(Rc::new(move || called.set(true)))).setup_with_classic_desktop_lifetime_with(
        &args,
        move |l| {
            builder_called.set(true);
            let raised = raised.clone();
            l.startup(move |_| raised.set(true));
        },
    );

    assert!(framework_init_called.get());
    assert!(lifetime_builder_called.get());
    assert!(!startup_raised.get());
}

#[test]
fn start_after_setup_with_classic_desktop_lifetime_should_not_raise_startup_twice() {
    let _scope = builder_scope();
    let lifetime: LifetimeSlot = Default::default();
    let raised = counter();

    let slot = lifetime.clone();
    let r = raised.clone();
    create_app_builder(None, None).setup_with_classic_desktop_lifetime_with(&[], move |l| {
        *slot.borrow_mut() = Some(l.clone());
        let r = r.clone();
        l.startup(move |_| r.set(r.get() + 1));
    });

    let lifetime = lifetime.borrow().clone().expect("the lifetime builder was called");

    assert_eq!(0, raised.get());

    let dispatcher = Dispatcher::ui_thread();
    let d = dispatcher.clone();
    dispatcher.post_local(move || d.exit_all_frames(), DispatcherPriority::DEFAULT);
    lifetime.start(&[]);

    assert_eq!(1, raised.get());
    lifetime.dispose();
}

// --- tests of this port -----------------------------------------------------

#[test]
fn the_lifetime_is_reachable_through_the_lifetime_contracts() {
    let _scope = builder_scope();
    let args = ["foo".to_string()];
    let builder = create_app_builder(None, None).setup_with_classic_desktop_lifetime(&args);

    let application = builder.instance().unwrap();
    assert!(Application::current().is_some_and(|current| current == application));
    let lifetime = application.application_lifetime().unwrap();
    let desktop = lifetime.as_classic_desktop_style_application_lifetime().unwrap();

    assert_eq!(desktop.args(), Some(vec!["foo".to_string()]));
    assert_eq!(desktop.shutdown_mode(), ShutdownMode::OnLastWindowClose);
    assert!(desktop.main_window().is_none());
    assert!(lifetime.as_controlled_application_lifetime().is_some());
    assert!(lifetime.as_setup_application_lifetime().is_some());
    assert!(lifetime.as_any().downcast_ref::<ClassicDesktopStyleApplicationLifetime>().is_some());

    lifetime.as_any().downcast_ref::<ClassicDesktopStyleApplicationLifetime>().unwrap().dispose();
}

#[test]
fn start_passes_the_arguments_and_shows_the_main_window() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_args(Some(vec!["a".to_string(), "b".to_string()]));
    let main_window = Window::new();
    lifetime.set_main_window(Some(main_window.clone()));

    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let s = seen.clone();
    lifetime.startup(move |e| *s.borrow_mut() = e.args().to_vec());
    let l = lifetime.clone();
    lifetime.exit(move |e| {
        assert!(l.windows().is_empty());
        e.set_application_exit_code(e.application_exit_code() + 1);
    });

    // Closing the main (and last) window from a job ends the loop.
    let w = main_window.clone();
    Dispatcher::ui_thread().post_local(move || w.close(), DispatcherPriority::DEFAULT);
    let exit_code = lifetime.start_with_own_args();

    assert_eq!(*seen.borrow(), vec!["a".to_string(), "b".to_string()]);
    assert_eq!(1, exit_code);
    assert!(main_window.platform_impl().is_none());
    lifetime.dispose();
}

#[test]
fn dispose_stops_tracking_windows_and_platform_shutdown_requests() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime_events = Rc::new(TestPlatformLifetimeEvents::default());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformLifetimeEventsImpl>()
        .to_constant(lifetime_events.clone() as Rc<dyn IPlatformLifetimeEventsImpl>);
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);
    assert_eq!(1, lifetime_events.shutdown_requested.len());

    lifetime.dispose();

    let window = shown_window();
    assert!(lifetime.windows().is_empty());
    assert!(lifetime_events.shutdown_requested.is_empty());
    assert_eq!(1, Rc::strong_count(&lifetime));
    window.close();
}

#[test]
fn platform_shutdown_request_closes_the_windows_for_an_os_shutdown() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lifetime_events = Rc::new(TestPlatformLifetimeEvents::default());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformLifetimeEventsImpl>()
        .to_constant(lifetime_events.clone() as Rc<dyn IPlatformLifetimeEventsImpl>);
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    setup(&lifetime);
    let has_exit = track_exit(&lifetime);

    let window = shown_window();
    let reason = Rc::new(Cell::new(None));
    let r = reason.clone();
    window.closing(move |e| r.set(Some((e.close_reason(), e.is_programmatic()))));

    let e = ShutdownRequestedEventArgs::with_is_os_shutdown(true);
    lifetime_events.raise_shutdown_requested(&e);

    assert_eq!(reason.get(), Some((crate::WindowCloseReason::OSShutdown, false)));
    assert!(has_exit.get());
    assert!(!e.cancel());
    // No main loop is running, so none is exited.
    assert!(!e.will_exit_main_loop());
    lifetime.dispose();
}

/// A dispatcher implementation without a run loop: the event loop belongs
/// to the host.
struct HostDrivenDispatcherImpl(ManagedDispatcherImpl);

impl IDispatcherImpl for HostDrivenDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        self.0.current_thread_is_loop_thread()
    }

    fn signal(&self) {
        self.0.signal()
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        self.0.signal_handle()
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        self.0.signaled()
    }

    fn timer(&self) -> &DispatcherImplEvent {
        self.0.timer()
    }

    fn now(&self) -> i64 {
        self.0.now()
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        self.0.update_timer(due_time_in_ms)
    }
}

#[test]
fn start_returns_after_the_startup_without_run_loops_and_shutdown_completes_later() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    Dispatcher::initialize_ui_thread_dispatcher(Rc::new(HostDrivenDispatcherImpl(ManagedDispatcherImpl::new(None))));
    assert!(!Dispatcher::ui_thread().supports_run_loops());

    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_main_window(Some(Window::new()));
    let startup_raised = counter();
    let r = startup_raised.clone();
    lifetime.startup(move |_| r.set(r.get() + 1));
    let exit_code = Rc::new(Cell::new(None));
    let c = exit_code.clone();
    lifetime.exit(move |e| c.set(Some(e.application_exit_code())));

    // Nothing ends a loop here: the call returns because none is run.
    assert_eq!(0, lifetime.start(&[]));

    assert_eq!(1, startup_raised.get());
    assert_eq!(1, lifetime.windows().len());
    assert!(lifetime.main_window().unwrap().is_visible());
    assert_eq!(exit_code.get(), None);

    // The application also runs without a main window handle held here.
    let application = Application::current().unwrap();
    application.run_window(lifetime.main_window().unwrap());
    application.run_with_main_window(Window::new);
    assert_eq!(2, lifetime.windows().len());

    let shutdown_started = flag();
    let s = shutdown_started.clone();
    Dispatcher::ui_thread().shutdown_started(move |_| s.set(true));

    lifetime.shutdown(7);

    assert_eq!(exit_code.get(), Some(7));
    assert!(lifetime.windows().is_empty());
    assert!(shutdown_started.get());
    lifetime.dispose();
}

// --- the desktop application extensions --------------------------------------

#[test]
fn run_window_shows_the_window_and_runs_until_it_is_closed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let application = Application::current().unwrap();
    let window = Window::new();

    let w = window.clone();
    Dispatcher::ui_thread().post_local(
        move || {
            assert!(w.is_visible());
            w.close();
        },
        DispatcherPriority::DEFAULT,
    );
    application.run_window(&window);

    assert!(window.platform_impl().is_none());
}

#[test]
fn run_with_main_window_creates_and_shows_the_window() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let application = Application::current().unwrap();
    let created: Rc<std::cell::RefCell<Option<Ref<Window>>>> = Default::default();

    let c = created.clone();
    Dispatcher::ui_thread().post_local(
        move || {
            let window = c.borrow().clone().unwrap();
            assert!(window.is_visible());
            window.close();
        },
        DispatcherPriority::DEFAULT,
    );
    let c = created.clone();
    application.run_with_main_window(move || {
        let window = Window::new();
        *c.borrow_mut() = Some(window.clone());
        window
    });

    assert!(created.borrow().as_ref().unwrap().platform_impl().is_none());
}

#[test]
fn run_runs_until_the_closeable_is_closed() {
    use ferroui_base::input::ICloseable;

    #[derive(Default)]
    struct Closeable(Rc<HandlerList<dyn Fn()>>);

    impl ICloseable for Closeable {
        fn closed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            let token = self.0.add(handler);
            let handlers = self.0.clone();
            Disposable::create(move || {
                handlers.remove(token);
            })
        }
    }

    let _app = UnitTestApplication::start(TestServices::new());
    let application = Application::current().unwrap();
    let closeable = Closeable::default();

    let handlers = closeable.0.clone();
    let ran = flag();
    let r = ran.clone();
    Dispatcher::ui_thread().post_local(
        move || {
            r.set(true);
            for (_, handler) in handlers.snapshot().iter() {
                handler();
            }
        },
        DispatcherPriority::DEFAULT,
    );
    application.run(&closeable);

    assert!(ran.get());
}
