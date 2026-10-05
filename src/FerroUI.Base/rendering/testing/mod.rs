//! Test doubles for the render contracts: a platform drawing context that
//! records what is drawn, render targets and a render interface built on
//! it. Available to tests of this crate and, with the `testing` feature, to
//! other crates.

mod mock_drawing_context_impl;
mod mock_platform_render_interface;

pub use mock_drawing_context_impl::{
    DrawingLog, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockRenderTargetBitmapImpl,
};
pub use mock_platform_render_interface::{
    MockGeometryImpl, MockGlyphRunImpl, MockPlatformRenderInterface, MockPlatformRenderInterfaceContext, MockRegion,
    MockRenderTarget, MockStreamGeometryImpl,
};

use crate::rendering::{IRenderLoop, IRenderLoopTask};
use std::sync::{Arc, Mutex};

/// A render loop that never ticks by itself: tests drive frames explicitly.
#[derive(Default)]
pub struct ManualRenderLoop {
    tasks: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
}

impl ManualRenderLoop {
    pub fn new() -> Arc<ManualRenderLoop> {
        Arc::new(ManualRenderLoop::default())
    }

    /// Runs every registered task once, on the calling thread.
    pub fn tick(&self) {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for task in tasks {
            task.render();
        }
    }

    /// The number of registered tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

impl IRenderLoop for ManualRenderLoop {
    fn add(&self, i: Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).push(i);
    }

    fn remove(&self, i: &Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).retain(|t| !Arc::ptr_eq(t, i));
    }

    fn runs_in_background(&self) -> bool {
        false
    }

    fn wakeup(&self) {}
}
