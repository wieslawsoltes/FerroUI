use super::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use crate::platform::{ILockedFramebuffer, RenderTargetSceneInfo};
use std::rc::Rc;

/// A surface that is rendered to through a locked framebuffer.
pub trait IFramebufferPlatformSurface: IPlatformRenderSurface {
    /// Creates the framebuffer render target of the surface.
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget>;
}

/// Properties of a locked framebuffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramebufferLockProperties {
    /// Whether the framebuffer still holds the contents of the previous
    /// frame.
    pub previous_frame_is_retained: bool,
}

/// The render target of a framebuffer surface.
pub trait IFramebufferRenderTarget: IPlatformRenderSurfaceRenderTarget {
    /// Provides a framebuffer descriptor for drawing.
    ///
    /// The contents of the framebuffer are presented when it is disposed.
    fn lock(&self, scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties);

    /// Whether the target keeps the frame contents between locks.
    fn retains_frame_contents(&self) -> bool {
        false
    }

    /// Releases the render target.
    fn dispose(&self);
}

type LockFramebuffer = dyn Fn(&RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties);

/// A framebuffer render target backed by a function.
pub struct FuncFramebufferRenderTarget {
    lock_framebuffer: Box<LockFramebuffer>,
    retains_frame_contents: bool,
}

impl FuncFramebufferRenderTarget {
    /// Creates a target from a function that locks the framebuffer.
    pub fn new(lock_framebuffer: impl Fn() -> Rc<dyn ILockedFramebuffer> + 'static) -> Self {
        Self {
            lock_framebuffer: Box::new(move |_| (lock_framebuffer(), FramebufferLockProperties::default())),
            retains_frame_contents: false,
        }
    }

    /// Creates a target from a function that receives the scene info and
    /// reports the lock properties.
    pub fn with_scene_info(
        lock_framebuffer: impl Fn(&RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties)
            + 'static,
        retains_frame_contents: bool,
    ) -> Self {
        Self { lock_framebuffer: Box::new(lock_framebuffer), retains_frame_contents }
    }
}

impl IPlatformRenderSurfaceRenderTarget for FuncFramebufferRenderTarget {}

impl IFramebufferRenderTarget for FuncFramebufferRenderTarget {
    fn lock(&self, scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        (self.lock_framebuffer)(scene_info)
    }

    fn retains_frame_contents(&self) -> bool {
        self.retains_frame_contents
    }

    fn dispose(&self) {}
}
