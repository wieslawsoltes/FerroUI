//! sample-testing
//!
//! The test harness of the small samples (not a port: the upstream samples have no tests),
//! what the tests of the catalog do for the catalog (`samples/ControlCatalog/tests`):
//!
//! - [`services`]: the test services of a sample: the services of a styled window with the
//!   Skia render interface and font manager and the HarfBuzz text shaper in place of the mock
//!   ones, and a global clock the test ticks ([`TestGlobalClock`]);
//! - [`documents`]: the bodies of the generated per-document tests (a document loads through
//!   the run-time loader; its class constructs and is shown; the class populated by its
//!   compiled markup is the tree the run-time loader populates);
//! - [`Shell`]: the real application of a sample, headless: its windows render frames through
//!   the compositor into memory ([`Frame`]), and take the input of a mouse;
//! - [`BindingReports`]: the events the bindings log, for the guard of the accepted reports.

mod binding_reports;
mod clock;
pub mod documents;
mod shell;
mod surface;

pub use binding_reports::{assert_accepted, BindingReports, Report};
pub use clock::TestGlobalClock;
pub use shell::Shell;
pub use surface::{Frame, FrameSurface};

use ferroui_controls::testing::{TestIconLoader, TestServices};
use std::rc::Rc;

/// The test services of a sample: the services of a styled window with the Skia render
/// interface and font manager and the HarfBuzz text shaper in place of the mock ones, `clock`
/// as the global clock and the icon loader of tests. The theme is left to the application of
/// the sample.
pub fn services(clock: Rc<TestGlobalClock>) -> TestServices {
    let mut services = TestServices::styled_window()
        .with_render_interface(Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)))
        .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
        .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()))
        .with_global_clock(clock)
        .with_icon_loader(Rc::new(TestIconLoader));
    services.theme = None;
    services
}
