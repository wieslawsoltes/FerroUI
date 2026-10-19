//! Port of `AppTests.cs`: how [`App`] composes the shell for each application lifetime.
//!
//! The reference stands in for the lifetimes with runtime proxies (`DispatchProxy`), since its
//! framework seals the lifetime interfaces; here the proxies are test types that implement the
//! traits of the lifetimes, and a member a test does not use panics as the reference's proxies
//! throw `NotSupportedException`.

use super::support::{holds_object, start_application, TemporaryDirectory};
use crate::services::{BrowserMachineMetricsService, LocalMachineMetricsService, ShowcaseSession};
use crate::views::{MainView, MainWindow};
use crate::App;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{
    ControlledApplicationLifetimeExitEventArgs, ControlledApplicationLifetimeStartupEventArgs, IApplicationLifetime,
    IClassicDesktopStyleApplicationLifetime, IControlledApplicationLifetime, ISingleViewApplicationLifetime, ShutdownRequestedEventArgs,
};
use ferroui_controls::{Control, ShutdownMode, Window};
use ferroui_reactive::disposables::Disposable;
use ferroui_splat::ModernDependencyResolver;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The handlers of the exit of a lifetime.
type ExitHandlers = Rc<RefCell<Vec<(u64, Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>)>>>;

/// Stands in for the desktop lifetime, so that the test can raise the exit without shutting down
/// the dispatcher of the test (`DesktopLifetimeProxy`).
#[derive(Default)]
pub struct DesktopLifetimeProxy {
    /// The handlers subscribed to the exit event (`_exit`).
    exit: ExitHandlers,
    /// The next token of a handler.
    next_token: RefCell<u64>,
    /// The window the app assigned (`MainWindow`).
    main_window: RefCell<Option<Ref<Window>>>,
}

impl DesktopLifetimeProxy {
    /// The lifetime (`Create()`).
    pub fn create() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The window the app assigned.
    pub fn assigned_main_window(&self) -> Option<Ref<Window>> {
        self.main_window.borrow().clone()
    }

    /// Raises the exit event the way a closing desktop lifetime does (`RaiseExit()`).
    pub fn raise_exit(&self) {
        let args = ControlledApplicationLifetimeExitEventArgs::new(0);
        let handlers: Vec<_> = self.exit.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(&args);
        }
    }

    /// The member the reference's proxy does not support (`NotSupportedException`).
    fn not_supported(member: &str) -> ! {
        panic!("NotSupportedException: {member}")
    }
}

impl IApplicationLifetime for DesktopLifetimeProxy {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_controlled_application_lifetime(&self) -> Option<&dyn IControlledApplicationLifetime> {
        Some(self)
    }

    fn as_classic_desktop_style_application_lifetime(&self) -> Option<&dyn IClassicDesktopStyleApplicationLifetime> {
        Some(self)
    }
}

impl IControlledApplicationLifetime for DesktopLifetimeProxy {
    fn startup(&self, _handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>) -> Rc<dyn IDisposable> {
        Self::not_supported("add_Startup")
    }

    /// `add_Exit` and `remove_Exit`.
    fn exit(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>) -> Rc<dyn IDisposable> {
        let token = {
            let mut next_token = self.next_token.borrow_mut();
            *next_token += 1;
            *next_token
        };
        self.exit.borrow_mut().push((token, handler));
        let handlers = self.exit.clone();
        Disposable::create(move || handlers.borrow_mut().retain(|(registered, _)| *registered != token))
    }

    fn shutdown(&self, _exit_code: i32) {
        Self::not_supported("Shutdown")
    }
}

impl IClassicDesktopStyleApplicationLifetime for DesktopLifetimeProxy {
    fn try_shutdown(&self, _exit_code: i32) -> bool {
        Self::not_supported("TryShutdown")
    }

    fn args(&self) -> Option<Vec<String>> {
        Self::not_supported("get_Args")
    }

    fn shutdown_mode(&self) -> ShutdownMode {
        Self::not_supported("get_ShutdownMode")
    }

    fn set_shutdown_mode(&self, _value: ShutdownMode) {
        Self::not_supported("set_ShutdownMode")
    }

    /// `get_MainWindow`.
    fn main_window(&self) -> Option<Ref<Window>> {
        self.main_window.borrow().clone()
    }

    /// `set_MainWindow`.
    fn set_main_window(&self, value: Option<Ref<Window>>) {
        *self.main_window.borrow_mut() = value;
    }

    fn windows(&self) -> Vec<Ref<Window>> {
        Self::not_supported("get_Windows")
    }

    fn shutdown_requested(&self, _handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        Self::not_supported("add_ShutdownRequested")
    }
}

/// Stands in for the single-view lifetime of the browser, Android and iOS
/// (`SingleViewLifetimeProxy`).
#[derive(Default)]
pub struct SingleViewLifetimeProxy {
    /// The view the app assigned (`MainView`).
    main_view: RefCell<Option<Ref<Control>>>,
}

impl SingleViewLifetimeProxy {
    /// The lifetime (`Create()`).
    pub fn create() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The view the app assigned.
    pub fn assigned_main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }
}

impl IApplicationLifetime for SingleViewLifetimeProxy {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}

impl ISingleViewApplicationLifetime for SingleViewLifetimeProxy {
    /// `get_MainView`.
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    /// `set_MainView`.
    fn set_main_view(&self, value: Option<Ref<Control>>) {
        *self.main_view.borrow_mut() = value;
    }
}

#[test]
fn attach_shell_single_view_lifetime_creates_main_view_with_shell_view_model() {
    let _app = start_application();
    let lifetime = SingleViewLifetimeProxy::create();

    App::attach_shell(Some(&*lifetime as &dyn IApplicationLifetime));

    let view = lifetime.assigned_main_view().and_then(|view| view.cast::<MainView>()).expect("the main view is a MainView");
    let shell = MainView::view_model(&view).expect("the main view has the shell view model");
    assert!(holds_object(&view.data_context(), &shell));
    assert_eq!(shell.session_status(), ShowcaseSession::UNAVAILABLE_STATUS);
    shell.dispose();
}

#[test]
fn attach_shell_desktop_lifetime_creates_main_window_with_shell_view_model() {
    const DATA_HOME_VARIABLE: &str = "XDG_DATA_HOME";
    let _app = start_application();
    let previous_data_home = std::env::var_os(DATA_HOME_VARIABLE);
    let directory = TemporaryDirectory::new();
    std::env::set_var(DATA_HOME_VARIABLE, directory.path());

    let lifetime = DesktopLifetimeProxy::create();
    App::attach_shell(Some(&*lifetime as &dyn IApplicationLifetime));

    let window = lifetime.assigned_main_window().and_then(|window| window.cast::<MainWindow>()).expect("the main window is a MainWindow");
    let shell = MainWindow::view_model(&window).expect("the main window has the shell view model");
    assert!(holds_object(&window.data_context(), &shell));
    assert_ne!(shell.session_status(), ShowcaseSession::UNAVAILABLE_STATUS);

    lifetime.raise_exit();

    match previous_data_home {
        Some(previous) => std::env::set_var(DATA_HOME_VARIABLE, previous),
        None => std::env::remove_var(DATA_HOME_VARIABLE),
    }
}

#[test]
fn attach_shell_without_metrics_service_throws() {
    let _app = start_application();
    let resolver = ModernDependencyResolver::new();

    let shell = App::create_shell(&resolver);

    let error = shell.err().expect("creating the shell fails");
    assert!(error.downcast_ref::<crate::services::InvalidOperationError>().is_some(), "{error}");
}

// Deviation (DEVIATIONS.md, samples): the test with arguments of the reference
// (`CreateMetricsService_Matches_Platform(bool, Type)`) is one test per argument set.
#[test]
fn create_metrics_service_matches_platform_browser() {
    let _app = start_application();
    let service: Rc<dyn Any> = App::create_metrics_service(true);
    assert!(service.is::<BrowserMachineMetricsService>());
}

#[test]
fn create_metrics_service_matches_platform_local() {
    let _app = start_application();
    let service: Rc<dyn Any> = App::create_metrics_service(false);
    assert!(service.is::<LocalMachineMetricsService>());
}

#[test]
fn attach_shell_no_lifetime_does_nothing() {
    let _app = start_application();
    App::attach_shell(None);
}
