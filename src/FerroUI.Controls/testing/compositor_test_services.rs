use super::{MockWindowImpl, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::presentation_source::IRendererFactory;
use ferroui_base::media::MediaContext;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::testing::{DrawingLog, ManualRenderLoop, MockPlatformRenderInterface};
use ferroui_base::threading::Dispatcher;
use ferroui_base::FerroLocator;
use std::rc::Rc;
use std::sync::Arc;

/// A unit test application whose top-levels render through a real
/// compositing renderer over a test compositor, instead of the null
/// renderer of [`UnitTestApplication`].
///
/// The compositor never renders by itself: [`run_jobs`](Self::run_jobs)
/// commits the pending changes and renders one frame, after which hit
/// testing sees the scene.
///
/// ```ignore
/// let services = CompositorTestServices::start(TestServices::styled_window());
/// let window_impl = MockWindowingPlatform::create_window_mock();
/// services.setup(&window_impl);
/// let window = Window::with_impl(window_impl);
/// window.show();
/// services.run_jobs();
/// ```
pub struct CompositorTestServices {
    // Declared first: dropped before the application scope.
    compositor: Rc<Compositor>,
    render_loop: Arc<ManualRenderLoop>,
    app: UnitTestApplicationScope,
}

impl CompositorTestServices {
    /// Starts a unit test application with `services` and without the null
    /// renderer factory, and creates the compositor of its top-levels.
    ///
    /// Composition needs a platform render interface: the mock one is
    /// registered when `services` has none.
    pub fn start(mut services: TestServices) -> CompositorTestServices {
        if services.render_interface.is_none() {
            let render_interface: Rc<dyn IPlatformRenderInterface> =
                MockPlatformRenderInterface::new(DrawingLog::new());
            services.render_interface = Some(render_interface);
        }
        let app = UnitTestApplication::start(services);
        // Hide the null renderer factory of the unit test application: the
        // renderer seam then creates a compositing renderer over the
        // compositor of the platform implementation.
        FerroLocator::current_mutable().bind::<dyn IRendererFactory>().to_func(|| None);

        let render_loop = ManualRenderLoop::new();
        let compositor = Self::create_dummy_compositor(Some(render_loop.clone()));
        CompositorTestServices { compositor, render_loop, app }
    }

    /// Creates a compositor that is driven by `render_loop` (by a render
    /// loop that never ticks when `None`) and commits through the media
    /// context of the dispatcher.
    pub fn create_dummy_compositor(render_loop: Option<Arc<ManualRenderLoop>>) -> Rc<Compositor> {
        Compositor::with_scheduler(
            render_loop.unwrap_or_else(ManualRenderLoop::new),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        )
    }

    /// The compositor of the top-levels of the test.
    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The render loop that drives the compositor.
    pub fn render_loop(&self) -> &Arc<ManualRenderLoop> {
        &self.render_loop
    }

    /// Makes `platform_impl` return the compositor of the test, so that the
    /// top-level created over it renders through a compositing renderer.
    pub fn setup(&self, platform_impl: &MockWindowImpl) {
        platform_impl.setup_compositor(Some(self.compositor.clone()));
    }

    /// Runs the dispatcher jobs (layout, the renderer update and the
    /// commit), renders one frame and runs the jobs the frame posted.
    pub fn run_jobs(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    /// Ends the unit test application.
    pub fn dispose(&self) {
        self.app.dispose();
    }
}
