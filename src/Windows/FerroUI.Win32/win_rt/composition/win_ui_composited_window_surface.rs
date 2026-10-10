//! The surface of a window in the Windows.UI.Composition mode: a drawing
//! surface of a composition graphics device over the device of Direct3D
//! 11 of the context, a rectangle of whose texture is drawn to for each
//! frame; and the blur effects the mode has.

use super::win_ui_composition_shared::WinUiCompositionVersions;
use crate::i_blur_host::BlurEffect;
use crate::platform_constants::Version;

/// Whether a system of the version has the effect in this mode.
pub(crate) fn is_blur_supported(windows_version: Version, effect: BlurEffect) -> bool {
    match effect {
        BlurEffect::None => true,
        BlurEffect::Acrylic => windows_version >= WinUiCompositionVersions::MIN_ACRYLIC_VERSION,
        BlurEffect::MicaLight => windows_version >= WinUiCompositionVersions::MIN_HOST_BACKDROP_VERSION,
        BlurEffect::MicaDark => windows_version >= WinUiCompositionVersions::MIN_HOST_BACKDROP_VERSION,
        _ => false,
    }
}

#[cfg(windows)]
pub(crate) use imp::WinUiCompositedWindowSurface;

#[cfg(windows)]
mod imp {
    use super::super::win_ui_composited_window::{Transaction, WinUiCompositedWindow};
    use super::super::win_ui_composition_shared::WinUiCompositionShared;
    use super::super::Required;
    use crate::direct_x::{
        IDirect3D11TexturePlatformSurface, IDirect3D11TexturePlatformSurface2, IDirect3D11TextureRenderTarget,
        IDirect3D11TextureRenderTarget2, IDirect3D11TextureRenderTargetRenderSession,
    };
    use crate::i_blur_host::{BlurEffect, ICompositionEffectsSurface};
    use crate::interop::unmanaged_methods::{POINT, SIZE};
    use crate::win32_platform::Win32Platform;
    use crate::win32_top_level_scene_info::Win32TopLevelSceneInfo;
    use crate::win_rt::{
        DirectXAlphaMode, DirectXPixelFormat, ICompositionDrawingSurface, ICompositionDrawingSurfaceInterop,
        ICompositionGraphicsDevice, ICompositionGraphicsDevice2, ICompositionSurface, ICompositor, ICompositorInterop,
    };
    use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
    use ferroui_base::platform::{IPlatformGraphicsContext, PlatformRenderTargetState, PlatformThemeVariant, RenderTargetSceneInfo};
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::{PixelPoint, PixelSize, RenderTargetCorruptedException};
    use ferroui_microcom::{ComPtr, Guid, HResult, IUnknown, Interface};
    use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
    use std::any::{Any, TypeId};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, PoisonError};

    /// `IID_ID3D11Texture2D`.
    const IID_ID3D11_TEXTURE2D: Guid = crate::direct_x::ID3D11Texture2D::IID;

    struct Inner {
        shared: Arc<WinUiCompositionShared>,
        info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
        /// `WinUICompositionBackdropCornerRadius` of the options. The
        /// reference reads the options when a render target is created, on
        /// the thread that renders; the options of the port are of the UI
        /// thread, which reads the value when the mode is registered.
        backdrop_corner_radius: Option<f32>,
        window: Mutex<Option<Arc<WinUiCompositedWindow>>>,
    }

    /// The surface. It is shared between the threads and holds what the
    /// threads share (the window, the shared state of the mode, and the
    /// composition tree once the thread that renders created it); the view
    /// the thread that renders asks for is a surface of its own over the
    /// same state.
    pub(crate) struct WinUiCompositedWindowSurface {
        inner: Arc<Inner>,
    }

    impl WinUiCompositedWindowSurface {
        pub fn new(
            shared: Arc<WinUiCompositionShared>,
            info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
            backdrop_corner_radius: Option<f32>,
        ) -> Arc<WinUiCompositedWindowSurface> {
            Arc::new(WinUiCompositedWindowSurface {
                inner: Arc::new(Inner { shared, info, backdrop_corner_radius, window: Mutex::new(None) }),
            })
        }

        fn create_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<WinUiCompositedWindowRenderTarget>, HResult> {
            let window = {
                let mut window = self.inner.window.lock().unwrap_or_else(PoisonError::into_inner);
                match &*window {
                    Some(window) => window.clone(),
                    None => {
                        let created = WinUiCompositedWindow::new(
                            self.inner.info.clone(),
                            self.inner.shared.clone(),
                            self.inner.backdrop_corner_radius,
                        )?;
                        *window = Some(created.clone());
                        created
                    }
                }
            };

            WinUiCompositedWindowRenderTarget::new(context.clone(), window, d3d_device, self.inner.shared.compositor())
        }

        /// Releases the composition tree of the window.
        pub fn dispose(&self) {
            let window = self.inner.window.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(window) = window {
                window.dispose();
            }
        }
    }

    impl IPlatformRenderSurface for WinUiCompositedWindowSurface {
        fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
            let view = || Rc::new(WinUiCompositedWindowSurface { inner: self.inner.clone() });
            if kind == TypeId::of::<dyn IDirect3D11TexturePlatformSurface2>() {
                let this: Rc<dyn IDirect3D11TexturePlatformSurface2> = view();
                return Some(Rc::new(this));
            }
            if kind == TypeId::of::<dyn IDirect3D11TexturePlatformSurface>() {
                let this: Rc<dyn IDirect3D11TexturePlatformSurface> = view();
                return Some(Rc::new(this));
            }
            None
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IDirect3D11TexturePlatformSurface for WinUiCompositedWindowSurface {
        fn create_render_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget>, HResult> {
            Ok(self.create_target(context, d3d_device)?)
        }
    }

    impl IDirect3D11TexturePlatformSurface2 for WinUiCompositedWindowSurface {
        fn create_render_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget2>, HResult> {
            Ok(self.create_target(context, d3d_device)?)
        }
    }

    impl ICompositionEffectsSurface for WinUiCompositedWindowSurface {
        fn is_blur_supported(&self, effect: BlurEffect) -> bool {
            super::is_blur_supported(Win32Platform::windows_version(), effect)
        }
    }

    /// The drawing surface of a render target as its three interfaces.
    #[derive(Clone)]
    struct Surface {
        surface: ComPtr<ICompositionSurface>,
        surface_interop: ComPtr<ICompositionDrawingSurfaceInterop>,
        _drawing_surface: ComPtr<ICompositionDrawingSurface>,
    }

    /// What a render target holds until it is disposed.
    struct Device {
        _d3d_device: ComPtr<IUnknown>,
        _compositor: ComPtr<ICompositor>,
        _interop: ComPtr<ICompositorInterop>,
        _composition_device: ComPtr<ICompositionGraphicsDevice>,
        composition_device2: ComPtr<ICompositionGraphicsDevice2>,
    }

    pub(crate) struct WinUiCompositedWindowRenderTarget {
        context: Rc<dyn IPlatformGraphicsContext>,
        window: Arc<WinUiCompositedWindow>,
        /// `None` once disposed.
        device: RefCell<Option<Device>>,
        surface: RefCell<Option<Surface>>,
        size: Cell<PixelSize>,
        lost: Cell<bool>,
        is_surface_support_transparency: Cell<bool>,
    }

    impl WinUiCompositedWindowRenderTarget {
        fn new(
            context: Rc<dyn IPlatformGraphicsContext>,
            window: Arc<WinUiCompositedWindow>,
            device: isize,
            compositor: &ComPtr<ICompositor>,
        ) -> Result<Rc<WinUiCompositedWindowRenderTarget>, HResult> {
            // SAFETY: the caller of the surface contract passes a pointer
            // to the device of Direct3D 11 of its display, which lives
            // through the call; the render target takes a reference of its
            // own.
            let d3d_device = unsafe { ComPtr::<IUnknown>::from_raw_add_ref(device as *mut IUnknown) }.ok_or(HResult::POINTER)?;
            let compositor = compositor.clone();
            let interop = compositor.cast::<ICompositorInterop>()?;
            let composition_device = interop.create_graphics_device(Some(&d3d_device)).required()?;
            let composition_device2 = composition_device.cast::<ICompositionGraphicsDevice2>()?;
            // A failure above releases what was created before it, as the
            // reference does in its `catch`.
            Ok(Rc::new(WinUiCompositedWindowRenderTarget {
                context,
                window,
                device: RefCell::new(Some(Device {
                    _d3d_device: d3d_device,
                    _compositor: compositor,
                    _interop: interop,
                    _composition_device: composition_device,
                    composition_device2,
                })),
                surface: RefCell::new(None),
                size: Cell::new(PixelSize::default()),
                lost: Cell::new(false),
                is_surface_support_transparency: Cell::new(false),
            }))
        }

        fn dispose_target(&self) {
            *self.surface.borrow_mut() = None;
            *self.device.borrow_mut() = None;
        }

        fn create_surface(&self, scene_info: &RenderTargetSceneInfo) -> Result<Surface, HResult> {
            let device = self.device.borrow();
            let device = device.as_ref().ok_or(HResult::OBJECTDISPOSED)?;
            let is_transparency = scene_info.transparency_level != CompositionTransparencyLevel::None;
            let surface_size = scene_info.size;

            // Do not use Premultiplied when the window is not Transparency. Because the Premultiplied AlphaMode will
            // increase the performance loss of DWM.
            let alpha_mode = if is_transparency { DirectXAlphaMode::Premultiplied } else { DirectXAlphaMode::Ignore };
            let drawing_surface = device
                .composition_device2
                .create_drawing_surface2(
                    SIZE { x: surface_size.width, y: surface_size.height },
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    alpha_mode,
                )
                .required()?;
            let surface = drawing_surface.cast::<ICompositionSurface>()?;
            let surface_interop = drawing_surface.cast::<ICompositionDrawingSurfaceInterop>()?;

            self.is_surface_support_transparency.set(is_transparency);
            self.size.set(surface_size);
            Ok(Surface { surface, surface_interop, _drawing_surface: drawing_surface })
        }

        fn current_state(&self) -> PlatformRenderTargetState {
            if self.context.is_lost() || self.lost.get() {
                PlatformRenderTargetState::CORRUPTED
            } else {
                PlatformRenderTargetState::READY
            }
        }

        fn begin_draw_scene(
            &self,
            scene_info: &RenderTargetSceneInfo,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
            if self.current_state().is_corrupted {
                return Err(RenderTargetCorruptedException::new());
            }
            // The lock is left when the transaction is dropped: at the end
            // of the session, or here when the frame cannot be begun.
            let transaction = self.window.begin_transaction();

            // A failure outside the two calls below is an exception that
            // is not the corrupted render target in the reference; the
            // contract of the port has the one error, with the failure of
            // the system as its cause.
            let failed = |error: HResult| RenderTargetCorruptedException::new_with_inner_exception(Rc::new(error));

            let is_transparency = scene_info.transparency_level != CompositionTransparencyLevel::None;

            let existing = self.surface.borrow().clone();
            let surface = match existing {
                Some(surface) if self.is_surface_support_transparency.get() == is_transparency => surface,
                _ => {
                    // Re-create the surface with correct alpha mode if the transparency support is not correct. This can
                    // happen when the transparency level is changed.
                    *self.surface.borrow_mut() = None;
                    let surface = self.create_surface(scene_info).map_err(failed)?;
                    *self.surface.borrow_mut() = Some(surface.clone());
                    surface
                }
            };

            let size = scene_info.size;
            let scale = scene_info.scaling;
            self.window.resize_if_needed(size).map_err(failed)?;
            let theme_variant = scene_info
                .platform_specific_scene_info
                .as_ref()
                .and_then(|info| info.downcast_ref::<Win32TopLevelSceneInfo>())
                .map_or(PlatformThemeVariant::Light, |info| info.theme_variant);
            self.window.apply_effects(scene_info.transparency_level, theme_variant).map_err(failed)?;
            self.window.set_surface(&surface.surface).map_err(failed)?;

            let lost = |error: HResult| {
                self.lost.set(true);
                failed(error)
            };
            if self.size.get() != size {
                surface.surface_interop.resize(POINT { x: size.width, y: size.height }).map_err(lost)?;
                self.size.set(size);
            }
            let mut iid = IID_ID3D11_TEXTURE2D;
            let mut texture = std::ptr::null_mut();
            // SAFETY: no update rectangle (the whole surface); the
            // identifier and the pointer the texture is written to are
            // values of this frame that live through the call.
            let off = unsafe { surface.surface_interop.begin_draw(std::ptr::null_mut(), &mut iid, &mut texture) }.map_err(lost)?;

            let offset = PixelPoint::new(off.x, off.y);
            // SAFETY: the call succeeded, so the pointer is the texture
            // asked for, with a reference the caller owns.
            let Some(texture) = (unsafe { ComPtr::<IUnknown>::from_raw(texture.cast()) }) else {
                let _ = surface.surface_interop.end_draw();
                return Err(failed(HResult::POINTER));
            };

            Ok(Rc::new(Session {
                texture_pointer: Cell::new(texture.as_ptr() as isize),
                state: RefCell::new(Some(SessionState {
                    texture,
                    surface_interop: surface.surface_interop.clone(),
                    transaction,
                })),
                size: self.size.get(),
                offset,
                scaling: scale,
            }))
        }
    }

    impl IPlatformRenderSurfaceRenderTarget for WinUiCompositedWindowRenderTarget {
        fn state(&self) -> PlatformRenderTargetState {
            self.current_state()
        }
    }

    impl IDirect3D11TextureRenderTarget for WinUiCompositedWindowRenderTarget {
        fn begin_draw(&self) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
            let info = self.window.window_info();
            let fallback_scene_info = RenderTargetSceneInfo::new(info.size(), info.scaling(), CompositionTransparencyLevel::None);
            self.begin_draw_scene(&fallback_scene_info)
        }

        fn dispose(&self) {
            self.dispose_target();
        }
    }

    impl IDirect3D11TextureRenderTarget2 for WinUiCompositedWindowRenderTarget {
        fn begin_draw(
            &self,
            scene_info: &RenderTargetSceneInfo,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
            self.begin_draw_scene(scene_info)
        }

        fn dispose(&self) {
            self.dispose_target();
        }
    }

    struct SessionState {
        texture: ComPtr<IUnknown>,
        surface_interop: ComPtr<ICompositionDrawingSurfaceInterop>,
        transaction: Transaction,
    }

    struct Session {
        /// `None` once disposed.
        state: RefCell<Option<SessionState>>,
        texture_pointer: Cell<isize>,
        size: PixelSize,
        offset: PixelPoint,
        scaling: f64,
    }

    impl IDirect3D11TextureRenderTargetRenderSession for Session {
        fn d3d11_texture2d(&self) -> isize {
            self.texture_pointer.get()
        }

        fn size(&self) -> PixelSize {
            self.size
        }

        fn offset(&self) -> PixelPoint {
            self.offset
        }

        fn scaling(&self) -> f64 {
            self.scaling
        }

        fn dispose(&self) {
            let Some(SessionState { texture, surface_interop, transaction }) = self.state.borrow_mut().take() else {
                return;
            };
            self.texture_pointer.set(0);
            drop(texture);
            // A failure to end the frame throws out of the disposal in the
            // reference, after the transaction was disposed; the contract
            // has no error here, and a device that is lost is found by the
            // next frame.
            let _ = surface_interop.end_draw();
            drop(surface_interop);
            drop(transaction);
        }
    }

    impl Drop for Session {
        fn drop(&mut self) {
            // A session that is dropped without being disposed still ends
            // its frame and leaves the lock.
            IDirect3D11TextureRenderTargetRenderSession::dispose(self);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the surface.
    use super::*;
    use crate::platform_constants::PlatformConstants;

    #[test]
    fn the_effects_of_the_mode_by_the_version_of_windows() {
        let windows10_1803 = Version { major: 10, minor: 0, build: 17134 };
        let windows11 = Version { major: 10, minor: 0, build: 22000 };
        let server2022 = Version { major: 10, minor: 0, build: 20348 };

        for version in [PlatformConstants::WINDOWS10, windows10_1803, server2022, windows11] {
            assert!(is_blur_supported(version, BlurEffect::None));
            // The mode has no Gaussian blur: the blur level is not
            // supported by a window.
            assert!(!is_blur_supported(version, BlurEffect::GaussianBlur));
        }

        assert!(!is_blur_supported(PlatformConstants::WINDOWS10, BlurEffect::Acrylic));
        assert!(is_blur_supported(windows10_1803, BlurEffect::Acrylic));
        assert!(is_blur_supported(server2022, BlurEffect::Acrylic));
        assert!(is_blur_supported(windows11, BlurEffect::Acrylic));

        for effect in [BlurEffect::MicaLight, BlurEffect::MicaDark] {
            assert!(!is_blur_supported(windows10_1803, effect));
            assert!(!is_blur_supported(server2022, effect));
            assert!(is_blur_supported(windows11, effect));
        }
    }
}
