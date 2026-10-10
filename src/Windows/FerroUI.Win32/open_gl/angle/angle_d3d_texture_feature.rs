//! Lets a context of ANGLE on Direct3D 11 render to a surface that hands
//! out textures of Direct3D 11: the surfaces of the composition modes.

use super::AngleWin32EglDisplay;
use crate::angle_options::PlatformApi;
use crate::direct_x::{
    DxgiErrorExtensions, IDirect3D11TexturePlatformSurface2, IDirect3D11TextureRenderTarget2,
    IDirect3D11TextureRenderTargetRenderSession, DXGI_ERROR,
};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{
    IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::RenderTargetCorruptedException;
use ferroui_microcom::HResult;
use ferroui_opengl::egl::{EglContext, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase};
use ferroui_opengl::surfaces::{IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::{IGlContext, IGlPlatformSurfaceRenderTargetFactory};
use std::any::TypeId;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The feature of one context.
///
/// The reference asks the context it is given for its display and tests
/// the type of the display. A context of the port holds the EGL display
/// and not what wraps it, so the feature, which is created for a context
/// with the display of that context, keeps both.
pub(crate) struct AngleD3DTextureFeature {
    context: Weak<EglContext>,
    angle: Rc<AngleWin32EglDisplay>,
}

impl AngleD3DTextureFeature {
    pub fn new(context: &Rc<EglContext>, angle: Rc<AngleWin32EglDisplay>) -> AngleD3DTextureFeature {
        AngleD3DTextureFeature { context: Rc::downgrade(context), angle }
    }

    fn texture_surface(surface: &Arc<dyn IPlatformRenderSurface>) -> Option<Rc<dyn IDirect3D11TexturePlatformSurface2>> {
        let kind = surface.try_get_surface_kind(TypeId::of::<dyn IDirect3D11TexturePlatformSurface2>())?;
        kind.downcast_ref::<Rc<dyn IDirect3D11TexturePlatformSurface2>>().cloned()
    }
}

/// Whether a failure of the system means that the device is lost.
pub(crate) fn is_device_lost(error: HResult) -> bool {
    DXGI_ERROR(error.0).is_device_lost_error()
}

/// Whether the cause of a corrupted render target is a lost device.
pub(crate) fn is_caused_by_device_loss(error: &RenderTargetCorruptedException) -> bool {
    error.inner_exception().and_then(|inner| inner.downcast_ref::<HResult>()).is_some_and(|inner| is_device_lost(*inner))
}

impl IGlPlatformSurfaceRenderTargetFactory for AngleD3DTextureFeature {
    fn can_render_to_surface(&self, context: &Rc<dyn IGlContext>, surface: &Arc<dyn IPlatformRenderSurface>) -> bool {
        context.as_any().is::<EglContext>()
            && self.angle.platform_api() == PlatformApi::DirectX11
            && Self::texture_surface(surface).is_some()
    }

    /// # Panics
    /// Panics when the context is not the context of the feature or the
    /// surface is not a texture surface (the failing casts of the
    /// reference), and when the render target cannot be created (the
    /// exception of the reference; a lost device is reported to the
    /// context first).
    fn create_render_target(
        &self,
        context: &Rc<dyn IGlContext>,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let ctx = match self.context.upgrade() {
            Some(ctx) if context.as_any().is::<EglContext>() => ctx,
            _ => panic!("Unable to cast the context to type 'EglContext'."),
        };
        let Some(texture_surface) = Self::texture_surface(surface) else {
            panic!("Unable to cast the surface to type 'IDirect3D11TexturePlatformSurface2'.");
        };

        let graphics_context: Rc<dyn IPlatformGraphicsContext> = ctx.clone();
        let target = self
            .angle
            .get_direct3d_device()
            .map_err(|error| error.to_string())
            .and_then(|device| {
                texture_surface.create_render_target(&graphics_context, device).map_err(|error| {
                    if is_device_lost(error) {
                        ctx.notify_context_lost();
                    }
                    error.to_string()
                })
            });
        match target {
            Ok(target) => Rc::new(RenderTargetWrapper {
                base: EglPlatformSurfaceRenderTargetBase::new(ctx),
                angle: self.angle.clone(),
                target,
            }),
            Err(error) => panic!("{error}"),
        }
    }
}

struct RenderTargetWrapper {
    base: EglPlatformSurfaceRenderTargetBase,
    angle: Rc<AngleWin32EglDisplay>,
    target: Rc<dyn IDirect3D11TextureRenderTarget2>,
}

/// Ends a frame that could not be begun, or was presented: the surface
/// over the texture, then the session of the texture, then the context.
fn release(
    context_lock: &Rc<dyn IDisposable>,
    session: Option<&Rc<dyn IDirect3D11TextureRenderTargetRenderSession>>,
    surface: Option<&Rc<ferroui_opengl::egl::EglSurface>>,
) {
    if let Some(surface) = surface {
        surface.dispose();
    }
    if let Some(session) = session {
        session.dispose();
    }
    context_lock.dispose();
}

impl EglPlatformSurfaceRenderTarget for RenderTargetWrapper {
    fn base(&self) -> &EglPlatformSurfaceRenderTargetBase {
        &self.base
    }

    /// # Panics
    /// Panics when the frame cannot be begun, as
    /// [`try_begin_draw_core`](Self::try_begin_draw_core), and when the
    /// render target of the surface is corrupted: the renderer calls the
    /// other member, which returns that as an error.
    fn begin_draw_core(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        match self.try_begin_draw_core(scene_info) {
            Ok(session) => session,
            Err(error) => panic!("{error}"),
        }
    }

    /// The corrupted render target of the surface is an error the
    /// compositor takes, as it catches the exception of the reference (a
    /// lost device is reported to the context first).
    ///
    /// # Panics
    /// Panics when the texture cannot be wrapped or the context cannot be
    /// made current with it (exceptions of the reference that its
    /// compositor does not catch).
    fn try_begin_draw_core(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        // TODO: use expectedPixelSize
        let context = self.base.context();
        let context_lock = IPlatformGraphicsContext::ensure_current(&**context);

        let session = match self.target.begin_draw(scene_info) {
            Ok(session) => session,
            Err(error) => {
                if is_caused_by_device_loss(&error) {
                    context.notify_context_lost();
                }
                release(&context_lock, None, None);
                return Err(error.into());
            }
        };

        let (offset, size) = (session.offset(), session.size());
        let surface = match self.angle.wrap_direct3d11_texture_with_offset(
            session.d3d11_texture2d(),
            offset.x,
            offset.y,
            size.width,
            size.height,
        ) {
            Ok(surface) => surface,
            Err(error) => {
                release(&context_lock, Some(&session), None);
                panic!("{error}");
            }
        };

        let on_finish: Rc<dyn Fn()> = {
            let (context_lock, session, surface) = (context_lock.clone(), session.clone(), surface.clone());
            Rc::new(move || release(&context_lock, Some(&session), Some(&surface)))
        };
        match self.base.begin_draw(&surface, size, session.scaling(), Some(on_finish), true, None, false) {
            Ok(rv) => Ok(rv),
            Err(error) => {
                release(&context_lock, Some(&session), Some(&surface));
                panic!("{error}");
            }
        }
    }

    fn dispose(&self) {
        self.target.dispose();
    }

    fn state(&self) -> PlatformRenderTargetState {
        if self.is_corrupted() {
            PlatformRenderTargetState::CORRUPTED
        } else {
            self.target.state()
        }
    }
}

impl IPlatformRenderSurfaceRenderTarget for RenderTargetWrapper {
    fn state(&self) -> PlatformRenderTargetState {
        EglPlatformSurfaceRenderTarget::state(self)
    }
}

impl IGlPlatformSurfaceRenderTarget for RenderTargetWrapper {
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        EglPlatformSurfaceRenderTarget::begin_draw(self, scene_info)
    }

    fn try_begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        EglPlatformSurfaceRenderTarget::try_begin_draw(self, scene_info)
    }

    fn dispose(&self) {
        EglPlatformSurfaceRenderTarget::dispose(self)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the feature.
    use super::*;
    use crate::direct_x::DXGI_ERROR;

    #[test]
    fn a_lost_device_is_told_from_other_failures() {
        assert!(is_device_lost(HResult(DXGI_ERROR::DXGI_ERROR_DEVICE_REMOVED.0)));
        assert!(is_device_lost(HResult(DXGI_ERROR::DXGI_ERROR_DEVICE_RESET.0)));
        assert!(!is_device_lost(HResult::FAIL));

        let lost = RenderTargetCorruptedException::new_with_inner_exception(Rc::new(HResult(
            DXGI_ERROR::DXGI_ERROR_DEVICE_HUNG.0,
        )));
        assert!(is_caused_by_device_loss(&lost));
        assert!(!is_caused_by_device_loss(&RenderTargetCorruptedException::new_with_inner_exception(Rc::new(
            HResult::INVALIDARG
        ))));
        assert!(!is_caused_by_device_loss(&RenderTargetCorruptedException::new()));
    }
}
