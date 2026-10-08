use super::ICompositionGlTexture;
use crate::IGlContext;
use ferroui_base::rendering::composition::{CompositionDrawingSurface, Compositor};
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// Represents an OpenGL context that can be used to draw into a [`CompositionDrawingSurface`].
/// The context is either shared with the compositor's rendering context or uses external
/// object interop to transfer rendered frames to the compositor.
pub trait ICompositionGlContext {
    /// The compositor this context is associated with.
    fn compositor(&self) -> Rc<Compositor>;

    /// The underlying OpenGL context. Make it current using
    /// [`IGlContext::make_current`] before issuing OpenGL calls.
    fn gl_context(&self) -> Rc<dyn IGlContext>;

    /// Checks if the context can still be used to transfer frames to the compositor. When
    /// this is false (e. g. because the compositor's GPU context was lost and recreated)
    /// all textures created by this context are no longer valid and the context should be
    /// disposed and recreated.
    fn is_valid_for_interop(&self) -> bool;

    /// Creates a texture that can be rendered to using the OpenGL context and then presented
    /// to the specified composition surface. Requires the OpenGL context to be current.
    ///
    /// `size` is the size of the texture in pixels.
    fn create_texture(&self, surface: &CompositionDrawingSurface, size: PixelSize) -> Rc<dyn ICompositionGlTexture>;

    /// Releases the context and its textures (`DisposeAsync`).
    fn dispose_async(&self) -> DispatcherTask<()>;
}
