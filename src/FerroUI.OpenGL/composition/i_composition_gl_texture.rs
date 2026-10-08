use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::ServerJobTask;
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// Represents an OpenGL texture that can be presented to a
/// [`CompositionDrawingSurface`](ferroui_base::rendering::composition::CompositionDrawingSurface).
///
/// Disposing the texture requires the associated OpenGL context to be current or to be
/// available to be made current (i. e. not being current on another thread). The disposal
/// of the texture is asynchronous since it can only be destroyed after the compositor
/// stops using it.
pub trait ICompositionGlTexture {
    /// The size of the texture in pixels.
    fn size(&self) -> PixelSize;

    /// Checks if the previous present operation has been completed and the texture can be
    /// used for drawing again.
    fn is_ready_for_draw(&self) -> bool;

    /// Begins a drawing operation. Requires the OpenGL context to be current. Dispose the
    /// returned lease to discard the frame, or call
    /// [`ICompositionGlTextureLease::present_async`] to send it to the compositor. The
    /// OpenGL context needs to be current when the lease is disposed or presented.
    ///
    /// # Panics
    /// Panics if the texture is not ready for drawing, see
    /// [`is_ready_for_draw`](Self::is_ready_for_draw).
    fn begin_draw(&self) -> Rc<dyn ICompositionGlTextureLease>;

    /// Releases the texture (`DisposeAsync`), once the compositor has stopped using it.
    fn dispose_async(&self) -> DispatcherTask<()>;
}

/// Represents a drawing operation in progress for an [`ICompositionGlTexture`]. Disposing
/// the lease discards the frame.
pub trait ICompositionGlTextureLease: IDisposable {
    /// Information about the texture to render to. Note that the texture might not be a
    /// regular GL_TEXTURE_2D, check the [`CompositionGlTextureInfo::target`] value.
    fn texture_info(&self) -> CompositionGlTextureInfo;

    /// Presents the texture to the composition surface. Requires the OpenGL context to be
    /// current. No further drawing to the texture is allowed until the returned task
    /// completes, see [`ICompositionGlTexture::is_ready_for_draw`].
    ///
    /// The task completes when the compositor has finished using the texture contents and
    /// it can be reused.
    fn present_async(&self) -> ServerJobTask<()>;
}

/// Describes an OpenGL texture that should be used as a render target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompositionGlTextureInfo {
    /// The OpenGL texture name.
    pub texture_id: i32,
    /// The texture target, e. g. GL_TEXTURE_2D or GL_TEXTURE_RECTANGLE.
    pub target: i32,
    /// The internal format of the texture.
    pub internal_format: i32,
    /// The size of the texture in pixels.
    pub size: PixelSize,
}

impl CompositionGlTextureInfo {
    pub fn new(texture_id: i32, target: i32, internal_format: i32, size: PixelSize) -> Self {
        Self { texture_id, target, internal_format, size }
    }
}
