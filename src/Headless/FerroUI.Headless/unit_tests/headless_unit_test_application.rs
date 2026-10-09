//! Port of `HeadlessUnitTestApplication.cs` of the upstream test library
//! (`tests/<the unit test library>` upstream): the test application running
//! on the real headless platform, set up through [`AppBuilder`] so that the
//! platform owns its own slice of the locator.
//!
//! This is deliberately not a flavour of the test services of the controls:
//! the headless platform registers services of its own (keyboard device,
//! clipboard, render loop, platform settings, hotkey config), and the unit
//! test application of the controls binds the same keys from its service
//! bag, so the two overwrite each other depending on when the platform
//! happens to initialize.
//!
//! What differs from the original:
//!
//! - The original binds the font manager of the test library after the
//!   platform services are set up: the default family is the Inter of the
//!   embedded font assets, and a typeface is the typeface of the headless
//!   platform. That font manager is ported with the tests of the Skia
//!   backend, over the test typeface of the base library, and the crate of
//!   the Inter assets is built on the Skia backend. Here the font manager
//!   of the headless platform stays, which creates the same typefaces from
//!   the font embedded in the platform.
//! - A test of the original is a test of a scoped test class, which enters
//!   a scope of the locator and resets the dispatcher around the test; the
//!   scope of the application holds the unit test scope of the dispatcher
//!   of its thread, so that the tests can run in parallel.

use crate::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_base::media::FontManager;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, FerroSynchronizationContext, UnitTestDispatcherScope};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Ref};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, IToolTipService, NewApplication};
use ferroui_themes_simple::SimpleTheme;
use std::cell::RefCell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

#[repr(C)]
pub(crate) struct HeadlessUnitTestApplication {
    base: Application,
}

ferro_class!(HeadlessUnitTestApplication: Application);
ferro_impl_classes!(HeadlessUnitTestApplication: FerroObjectImpl);

impl NewApplication for HeadlessUnitTestApplication {
    fn new_application() -> Ref<Self> {
        let this = instantiate(Self { base: Application::construct() });
        this.styles().add(SimpleTheme::new().as_style());
        this
    }
}

impl ApplicationImpl for HeadlessUnitTestApplication {}

impl HeadlessUnitTestApplication {
    pub(crate) fn start(options: Option<FerroHeadlessPlatformOptions>) -> Scope {
        let dispatcher = Dispatcher::unit_test_scope();
        let scope = FerroLocator::enter_scope();
        let old_context = FerroSynchronizationContext::current();

        if let Err(ex) = catch_unwind(AssertUnwindSafe(|| {
            Dispatcher::reset_before_unit_tests();

            AppBuilder::configure::<HeadlessUnitTestApplication>()
                // Popups default to dedicated top-levels here, matching the desktop platforms
                // and the app used by the unit tests of the headless platform.
                .use_headless(options.unwrap_or(FerroHeadlessPlatformOptions {
                    overlay_popups: false,
                    ..FerroHeadlessPlatformOptions::default()
                }))
                .setup_unsafe();
        })) {
            scope.dispose();
            resume_unwind(ex);
        }

        Scope { state: RefCell::new(Some((scope, old_context, dispatcher))) }
    }
}

type ScopeState = (Rc<dyn IDisposable>, Option<Arc<FerroSynchronizationContext>>, UnitTestDispatcherScope);

pub(crate) struct Scope {
    state: RefCell<Option<ScopeState>>,
}

impl Scope {
    /// Runs pending dispatcher jobs. Headless top-levels post activation, resizes and rendering,
    /// so tests have to let those settle between acts. (The original ties the run to the
    /// cancellation token of the test, which has no counterpart.)
    pub(crate) fn run_jobs(&self, priority: Option<DispatcherPriority>) {
        Dispatcher::ui_thread().run_jobs(priority);
    }

    pub(crate) fn dispose(&self) {
        let Some((locator_scope, old_context, dispatcher)) = self.state.borrow_mut().take() else { return };

        // Do not run more user code while a failed test unwinds.
        if !std::thread::panicking() {
            if Dispatcher::ui_thread().check_access() {
                Dispatcher::ui_thread().run_jobs(None);
            }

            let tool_tip_service = FerroLocator::current().get_service::<dyn IToolTipService>();
            if let Some(tool_tip_service) = tool_tip_service.as_ref().and_then(|service| service.as_tool_tip_service())
            {
                tool_tip_service.dispose();
            }
            if let Some(font_manager) = FerroLocator::current().get_service::<FontManager>() {
                font_manager.dispose();
            }
            // The original disposes the input manager too (`IInputManager as IDisposable`), which
            // completes its observables. The service is reached through its interface, which
            // has no disposal here: the input manager goes with the scope of the locator.

            Dispatcher::reset_for_unit_tests();
        }
        locator_scope.dispose();
        Dispatcher::reset_before_unit_tests();
        FerroSynchronizationContext::set_current(old_context);
        drop(dispatcher);
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.dispose();
    }
}
