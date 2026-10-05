//! The scopes the suites run in: the counterpart of the test base class of
//! the XAML suites and of `UnitTestApplication.Start(TestServices.X)`.

use std::rc::Rc;

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::FerroLocator;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Control;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

pub use ferroui_controls::testing::TestServices;

/// The scope of a test of a class that derives from the XAML test base:
/// its own dispatcher, loaded queue and service locator scope, with the
/// run-time loader registered as the loader of documents without compiled
/// markup. Dropping it ends the scope.
pub struct XamlTestScope {
    locator: Option<Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

/// Starts the scope of a XAML test (the constructor of the test base
/// class). Hold the result for the whole test:
///
/// ```ignore
/// let _base = xaml_test_base();
/// ```
pub fn xaml_test_base() -> XamlTestScope {
    crate::register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    FerroRuntimeXamlLoader::register();
    XamlTestScope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for XamlTestScope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

/// `UnitTestApplication.Start(services)`, with the run-time loader
/// registered in the scope of the application. Dropping the result ends the
/// application.
pub fn unit_test_application(services: TestServices) -> UnitTestApplicationScope {
    crate::register_types();
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    scope
}

/// `UnitTestApplication.Start()`: an application without services.
pub fn unit_test_application_without_services() -> UnitTestApplicationScope {
    unit_test_application(TestServices::new())
}

/// `UnitTestApplication.Start(TestServices.StyledWindow)`.
pub fn styled_window_application() -> UnitTestApplicationScope {
    unit_test_application(TestServices::styled_window())
}

/// `UnitTestApplication.Start(TestServices.MockWindowingPlatform)`.
pub fn mock_windowing_platform_application() -> UnitTestApplicationScope {
    unit_test_application(TestServices::mock_windowing_platform())
}

/// `UnitTestApplication.Start(TestServices.MockPlatformRenderInterface)`.
pub fn mock_platform_render_interface_application() -> UnitTestApplicationScope {
    unit_test_application(TestServices::mock_platform_render_interface())
}
