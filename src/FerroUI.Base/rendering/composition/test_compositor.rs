//! A compositor for the tests of the composition API that need no visual
//! tree (the reference `CompositorTestServices` without its top-level).

use super::server::{IAnimatedServerObject, IServerObject};
use super::Compositor;
use crate::media::MediaContext;
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::threading::Dispatcher;
use std::rc::Rc;
use std::sync::Arc;

/// A compositor driven by a manual render loop, over the mock render
/// interface.
pub(crate) struct TestCompositor {
    // Declared first: dropped before the scopes below.
    pub compositor: Rc<Compositor>,
    pub render_loop: Arc<ManualRenderLoop>,
    _locator_scope: Rc<dyn crate::reactive::IDisposable>,
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
}

impl TestCompositor {
    pub fn new() -> TestCompositor {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, _) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        TestCompositor { compositor, render_loop, _locator_scope: locator_scope, _dispatcher_scope: dispatcher_scope }
    }

    /// Commits the pending changes and renders a frame (`RunJobs`).
    pub fn run_jobs(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    /// The server object of type `T` with the given id, after a commit.
    pub fn server<T: IServerObject>(&self, id: super::server::ServerObjectId) -> Rc<T> {
        self.compositor.server().get::<T>(id).expect("the server object exists")
    }

    /// The animatable server object with the given id, after a commit.
    pub fn animated(&self, id: super::server::ServerObjectId) -> Rc<dyn IAnimatedServerObject> {
        self.compositor.server().get_animated_object(id).expect("the server object exists")
    }
}
