//! A surface that is rendered to through textures of Direct3D 11: what the
//! composition modes give a window, and what a context of ANGLE renders to
//! through `AngleD3DTextureFeature`.

use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{IPlatformGraphicsContext, RenderTargetSceneInfo};
use ferroui_base::{PixelPoint, PixelSize, RenderTargetCorruptedException};
use ferroui_microcom::HResult;
use std::rc::Rc;

/// A surface whose render target hands out a texture of Direct3D 11 for
/// each frame.
///
/// A surface announces the kind through its surface kinds:
/// `try_get_surface_kind(TypeId::of::<dyn IDirect3D11TexturePlatformSurface>())`
/// holds an `Rc<dyn IDirect3D11TexturePlatformSurface>`.
pub trait IDirect3D11TexturePlatformSurface: IPlatformRenderSurface {
    /// The render target of the surface for a device of Direct3D 11 (a
    /// pointer to its `ID3D11Device`). A failure of the system is the
    /// `COMException` of the reference.
    fn create_render_target(
        &self,
        graphics_context: &Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget>, HResult>;
}

/// The surface kind whose render target is told the scene of a frame.
pub trait IDirect3D11TexturePlatformSurface2: IPlatformRenderSurface {
    fn create_render_target(
        &self,
        graphics_context: &Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget2>, HResult>;
}

pub trait IDirect3D11TextureRenderTarget: IPlatformRenderSurfaceRenderTarget {
    fn begin_draw(&self) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException>;

    /// Releases the render target.
    fn dispose(&self);
}

pub trait IDirect3D11TextureRenderTarget2: IPlatformRenderSurfaceRenderTarget {
    fn begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException>;

    /// Releases the render target.
    fn dispose(&self);
}

/// A frame: the texture to draw into, until the session is disposed.
pub trait IDirect3D11TextureRenderTargetRenderSession {
    /// A pointer to the `ID3D11Texture2D` of the frame, which the session
    /// owns.
    fn d3d11_texture2d(&self) -> isize;

    /// The size of the frame in pixels.
    fn size(&self) -> PixelSize;

    /// Where in the texture the frame begins.
    fn offset(&self) -> PixelPoint;

    fn scaling(&self) -> f64;

    /// Ends the frame, which presents it.
    fn dispose(&self);
}
