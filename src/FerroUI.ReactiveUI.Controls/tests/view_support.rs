//! The helpers of the ported tests of the views, the view hosts, the hooks and the builder: `TestExecutor.cs` (the setup of ReactiveUI before each
//! test), `ControlledLifetimeFixture.cs` (a controlled application lifetime whose exit a test
//! raises), and the window helpers of the tests.
//!
//! The reference's tests run on the headless platform; here they run in the unit test application
//! of the framework, with the services of a styled window (windows that show, and the test theme).

#![allow(dead_code)]

use crate::{register_binding_integration, AutoDataTemplateBindingHook, FerroActivationForViewFetcher};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::application_lifetimes::{
    ControlledApplicationLifetimeExitEventArgs, ControlledApplicationLifetimeStartupEventArgs, IApplicationLifetime,
    IControlledApplicationLifetime,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{AppBuilder, Control, Window};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::{IActivationForViewFetcher, IPropertyBindingHook, RxAppBuilder, RxSuspension};
use ferroui_splat::{IMutableDependencyResolver, MutableDependencyResolverExt};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// The application of a test: the unit test application, and ReactiveUI built as the reference's
/// test executor builds it.
pub struct TestApp {
    _scope: UnitTestApplicationScope,
}

/// Starts the unit test application of a test and builds ReactiveUI with the registrations of
/// FerroUI (`TestExecutor.ExecuteTest`): the activation fetcher, the data template hook,
/// the binding integration, the suspension host and the core services.
pub fn start() -> TestApp {
    let scope = UnitTestApplication::start(TestServices::styled_window());
    build_reactive_ui();
    TestApp { _scope: scope }
}

/// Resets the state of ReactiveUI and builds it as the reference's test executor does
/// (`ReactiveUIBuilder.ResetBuilderStateForTests()` and the build that follows).
pub fn build_reactive_ui() {
    reset_reactive_ui();
    let builder = RxAppBuilder::create_reactive_ui_builder();
    let _ = builder
        .with_registration(|splat: &dyn IMutableDependencyResolver| {
            splat.register_constant::<dyn IActivationForViewFetcher>(Rc::new(FerroActivationForViewFetcher::new()));
            splat.register_constant::<dyn IPropertyBindingHook>(Rc::new(AutoDataTemplateBindingHook::new()));
            register_binding_integration(splat);
        })
        .with_suspension_host()
        .with_core_services()
        .build_app();
}

/// Resets the state of ReactiveUI: the builder, the locator and the suspension host
/// (`ReactiveUIBuilder.ResetBuilderStateForTests()`).
pub fn reset_reactive_ui() {
    RxAppBuilder::reset_for_testing();
    RxSuspension::reset_for_testing();
}

/// `Dispatcher.UIThread.RunJobs()`.
pub fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// `new Window { Content = content }`.
pub fn window_with(content: Option<Ref<Control>>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(content.map(Control::boxed));
    window
}

/// The control the content of a host or window holds, if it holds one.
pub fn content_control(content: Option<BoxedValue>) -> Option<Ref<Control>> {
    content.as_ref().and_then(Control::from_boxed)
}

/// A string as a content (`"default"`).
pub fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_owned()))
}

/// Whether a content is the string `value`.
pub fn is_text(content: &Option<BoxedValue>, value: &str) -> bool {
    content.as_ref().and_then(|content| content.downcast_ref::<String>().cloned()).as_deref() == Some(value)
}

/// Whether two view models are the same object (`IsSameReferenceAs`).
pub fn same<T: ?Sized, U: ?Sized>(first: &Rc<T>, second: &Rc<U>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

/// Whether an untyped view model is the object `expected`.
pub fn is_view_model(view_model: Option<Rc<dyn Any>>, expected: &Rc<dyn Any>) -> bool {
    view_model.is_some_and(|view_model| same(&view_model, expected))
}

/// Whether two application builders are the same builder (`IsSameReferenceAs`): a callback added
/// through `second` is one of the callbacks of `first`. Only the callbacks of `after_setup` run,
/// which the tests do not otherwise use.
pub fn same_builder(first: &AppBuilder, second: &AppBuilder) -> bool {
    let marker = Rc::new(Cell::new(false));
    let seen = marker.clone();
    let _ = second.after_setup(move |_| seen.set(true));
    (first.after_setup_callback())(first);
    marker.get()
}

/// A controlled desktop lifetime whose exit a test raises (`ControlledLifetimeFixture`).
#[derive(Default)]
pub struct ControlledLifetime {
    startup: Rc<HandlerList<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>>,
    exit: Rc<HandlerList<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>>,
}

impl ControlledLifetime {
    /// The lifetime.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Raises the exit event (`RaiseExit`).
    pub fn raise_exit(&self) {
        self.raise_exit_with(0);
    }

    /// Raises the exit event with `exit_code`.
    pub fn raise_exit_with(&self, exit_code: i32) {
        let args = ControlledApplicationLifetimeExitEventArgs::new(exit_code);
        for (_, handler) in self.exit.snapshot().iter() {
            handler(&args);
        }
    }
}

impl IApplicationLifetime for ControlledLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_controlled_application_lifetime(&self) -> Option<&dyn IControlledApplicationLifetime> {
        Some(self)
    }
}

impl IControlledApplicationLifetime for ControlledLifetime {
    fn startup(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.startup.add(handler);
        let handlers = self.startup.clone();
        Disposable::create(move || {
            let _ = handlers.remove(token);
        })
    }

    fn exit(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.exit.add(handler);
        let handlers = self.exit.clone();
        Disposable::create(move || {
            let _ = handlers.remove(token);
        })
    }

    fn shutdown(&self, exit_code: i32) {
        self.raise_exit_with(exit_code);
    }
}

/// A lifetime that is not a controlled one (`CreateUnsupportedLifetime`).
pub struct UnsupportedLifetime;

impl IApplicationLifetime for UnsupportedLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
