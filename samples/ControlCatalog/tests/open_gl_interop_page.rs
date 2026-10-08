//! Tests of the OpenGL interop page.
//!
//! Not ports: the upstream sample has no tests. The applications of the tests have no
//! OpenGL interop with the compositor (as the catalog on a platform that renders with
//! another API), so what is tested is the page without a context.

use super::support::*;
use crate::pages::open_gl::GlPageKnobs;
use crate::pages::OpenGlInteropPage;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{Control, Window};

const NOT_AVAILABLE: &str = "Compositor OpenGL interop is not available on this platform";

#[test]
fn the_interop_page_attaches_and_detaches_without_a_compositor() {
    let _app = start_catalog_application();
    let page = OpenGlInteropPage::new();
    let knobs = page.get_control::<GlPageKnobs>("Knobs");
    let viewport = page.get_control::<Control>("Viewport");

    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(viewport.is_attached_to_visual_tree());
    // No context was created: the page shows no information of one.
    let info = knobs.info();
    assert!(info.is_empty() || info == NOT_AVAILABLE, "{info}");

    // The handler of the knobs runs; without a context nothing is drawn.
    knobs.set_yaw(1.0);
    knobs.set_disco(0.5);
    Dispatcher::ui_thread().run_jobs(None);

    window.close();
    assert!(!viewport.is_attached_to_visual_tree());
}

#[test]
fn the_interop_page_says_that_interop_is_not_available() {
    let services = start_catalog_compositor_application();
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(1100.0, 800.0);
    services.setup(&window_impl);
    let page = OpenGlInteropPage::new();
    let knobs = page.get_control::<GlPageKnobs>("Knobs");

    let window = Window::with_impl(window_impl.clone());
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    // The viewport has a composition visual, and its compositor no GPU interop.
    assert_eq!(knobs.info(), NOT_AVAILABLE);

    knobs.set_pitch(1.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(knobs.info(), NOT_AVAILABLE);

    window.close();
}
