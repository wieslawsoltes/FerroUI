//! An OpenGL surface over a composition swap chain a host owns (a swap
//! chain panel of another toolkit): the host says the size and the scaling,
//! and is handed the swap chain when the first render target creates it.

use ferroui_microcom::Guid;

// QI for IDXGISwapChain2 fails on some Windows builds even though
// IDXGISwapChain3/4 succeed. Use IDXGISwapChain3 which inherits from
// IDXGISwapChain2 and has SetMatrixTransform at the same vtable slot.
#[allow(dead_code)] // Queried on Windows.
pub(crate) const IDXGI_SWAP_CHAIN3_GUID: Guid =
    Guid::new(0x94d9_9bdb, 0xf1f8, 0x4ab0, [0xb2, 0x36, 0x7d, 0xa0, 0x17, 0x0e, 0xda, 0xb1]);

/// The slot of `IDXGISwapChain2::SetMatrixTransform` in its vtable:
/// `IUnknown` (3), `IDXGIObject` (4), `IDXGIDeviceSubObject` (1),
/// `IDXGISwapChain` (10), `IDXGISwapChain1` (11), then `SetSourceSize`,
/// `GetSourceSize`, `SetMaximumFrameLatency`, `GetMaximumFrameLatency` and
/// `GetFrameLatencyWaitableObject` (5).
pub(crate) const SET_MATRIX_TRANSFORM_SLOT: usize = 3 + 4 + 1 + 10 + 11 + 5;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) struct DXGI_MATRIX_3X2_F {
    pub _11: f32,
    pub _12: f32,
    pub _21: f32,
    pub _22: f32,
    pub _31: f32,
    pub _32: f32,
}

/// The transform that undoes the scaling of the host: the swap chain has
/// the size in pixels, and the host stretches it by its scaling. `None`
/// for a scaling that is not positive.
pub(crate) fn inverse_scale_transform(scaling: f64) -> Option<DXGI_MATRIX_3X2_F> {
    if scaling <= 0.0 {
        return None;
    }

    Some(DXGI_MATRIX_3X2_F { _11: 1.0 / scaling as f32, _22: 1.0 / scaling as f32, ..Default::default() })
}

#[cfg(windows)]
#[allow(unused_imports)] // Created by a host of another toolkit, which the port does not have yet.
pub(crate) use imp::{SwapChainGlRenderTarget, SwapChainGlSurface};

#[cfg(windows)]
#[allow(dead_code)] // As above: the tests construct it.
mod imp {
    use super::super::{AngleContextDisplay, AngleWin32EglDisplay};
    use super::{inverse_scale_transform, DXGI_MATRIX_3X2_F, IDXGI_SWAP_CHAIN3_GUID, SET_MATRIX_TRANSFORM_SLOT};
    use crate::direct_x::{
        IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, DXGI_ALPHA_MODE, DXGI_FORMAT, DXGI_SAMPLE_DESC, DXGI_SWAP_CHAIN_DESC1,
        DXGI_SWAP_EFFECT,
    };
    use crate::sync_root::SharedCom;
    use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
    use ferroui_base::platform::{
        IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo,
    };
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::PixelSize;
    use ferroui_microcom::{ComPtr, Guid, HResult, IUnknown, Interface, RawHResult};
    use ferroui_opengl::egl::{
        EglContext, EglGlPlatformSurfaceBase, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase, EglSurface,
    };
    use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
    use ferroui_opengl::IGlContext;
    use std::any::{Any, TypeId};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, PoisonError};

    #[link(name = "dxgi", kind = "raw-dylib")]
    extern "system" {
        fn CreateDXGIFactory2(flags: u32, riid: *const Guid, pp_factory: *mut *mut c_void) -> RawHResult;
    }

    /// `DXGI_USAGE_RENDER_TARGET_OUTPUT` (`DxgiRenderTarget` of the
    /// reference).
    const DXGI_USAGE_RENDER_TARGET_OUTPUT: u32 = 0x0000_0020;

    /// The swap chain as its first and as its third version.
    struct SwapChain {
        swap_chain: SharedCom<IDXGISwapChain1>,
        swap_chain3: SharedCom<IUnknown>,
    }

    struct Inner {
        get_size_func: Arc<dyn Fn() -> PixelSize + Send + Sync>,
        get_scaling_func: Arc<dyn Fn() -> f64 + Send + Sync>,
        set_swap_chain_callback: Arc<dyn Fn(isize) + Send + Sync>,
        swap_chain: Mutex<Option<SwapChain>>,
    }

    /// The surface. It is shared between the threads; the view a thread
    /// asks for is a surface of its own over the same state. The three
    /// functions of the host are called on the thread that renders.
    pub(crate) struct SwapChainGlSurface {
        inner: Arc<Inner>,
    }

    impl SwapChainGlSurface {
        /// `set_swap_chain_callback` is handed the pointer of the swap
        /// chain (an `IDXGISwapChain1`, not counted for the callee) when
        /// the first render target has created it.
        pub fn new(
            get_size_func: Arc<dyn Fn() -> PixelSize + Send + Sync>,
            get_scaling_func: Arc<dyn Fn() -> f64 + Send + Sync>,
            set_swap_chain_callback: Arc<dyn Fn(isize) + Send + Sync>,
        ) -> Arc<SwapChainGlSurface> {
            Arc::new(SwapChainGlSurface {
                inner: Arc::new(Inner { get_size_func, get_scaling_func, set_swap_chain_callback, swap_chain: Mutex::new(None) }),
            })
        }

        /// Sets the transform of the swap chain that undoes the scaling.
        /// Nothing without a swap chain or for a scaling that is not
        /// positive; a failure of the system is the exception of the
        /// reference.
        pub(crate) fn set_inverse_scale_transform(&self, scaling: f64) -> Result<(), HResult> {
            let swap_chain = self.inner.swap_chain.lock().unwrap_or_else(PoisonError::into_inner);
            let (Some(swap_chain), Some(mut inverse_scale)) = (swap_chain.as_ref(), inverse_scale_transform(scaling)) else {
                return Ok(());
            };

            let this = swap_chain.swap_chain3.as_ptr().cast::<c_void>();
            // SAFETY: the pointer is a live `IDXGISwapChain3` (the query
            // for that interface answered with it), whose vtable has
            // `SetMatrixTransform` of the second version at the slot named
            // above, with this signature; the matrix lives through the
            // call.
            HResult::check(unsafe {
                let vtable = *this.cast::<*const *const c_void>();
                let set_matrix_transform: unsafe extern "system" fn(*mut c_void, *mut DXGI_MATRIX_3X2_F) -> RawHResult =
                    std::mem::transmute(*vtable.add(SET_MATRIX_TRANSFORM_SLOT));
                set_matrix_transform(this, &mut inverse_scale)
            })
        }

        fn create_swap_chain(&self, display: &AngleWin32EglDisplay) -> Result<ComPtr<IDXGISwapChain1>, String> {
            let failed = |call: &'static str| move |error: HResult| format!("{call} failed: {error}");
            let d3d_device_ptr = display.get_direct3d_device().map_err(|error| error.to_string())?;
            // SAFETY: the display answers with a pointer to its device of
            // Direct3D, which lives as long as the display; a reference is
            // taken for the time of the query below.
            let d3d_device = unsafe { ComPtr::<IUnknown>::from_raw_add_ref(d3d_device_ptr as *mut IUnknown) }
                .ok_or_else(|| "The display has no Direct3D device".to_owned())?;

            let dxgi_device = d3d_device.cast::<IDXGIDevice>().map_err(failed("QueryInterface(IDXGIDevice)"))?;
            drop(d3d_device);

            let factory_guid = IDXGIFactory2::IID;
            let mut factory_ptr = std::ptr::null_mut();
            // SAFETY: the identifier is the one of the interface the
            // result is read as, and the pointer is valid for the one
            // pointer written.
            HResult::check(unsafe { CreateDXGIFactory2(0, &factory_guid, &mut factory_ptr) })
                .map_err(failed("CreateDXGIFactory2"))?;
            // SAFETY: the call succeeded, so the pointer is null or a
            // factory whose reference this function owns.
            let dxgi_factory = unsafe { ComPtr::<IDXGIFactory2>::from_raw(factory_ptr.cast()) }
                .ok_or_else(|| "CreateDXGIFactory2 returned no factory".to_owned())?;

            let pixel_size = (self.inner.get_size_func)();
            let mut desc = DXGI_SWAP_CHAIN_DESC1 {
                format: DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
                sample_desc: DXGI_SAMPLE_DESC { count: 1, quality: 0 },
                buffer_usage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                buffer_count: 2,
                swap_effect: DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                alpha_mode: DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_PREMULTIPLIED,
                width: pixel_size.width as u32,
                height: pixel_size.height as u32,
                flags: 0,
                ..Default::default()
            };

            let device_unknown: &IUnknown = &dxgi_device;
            // SAFETY: the description is a structure of this frame that
            // lives through the call; no output.
            unsafe { dxgi_factory.create_swap_chain_for_composition(Some(device_unknown), &mut desc, None) }
                .map_err(failed("IDXGIFactory2::CreateSwapChainForComposition"))?
                .ok_or_else(|| "IDXGIFactory2::CreateSwapChainForComposition returned no swap chain".to_owned())
        }

        /// Creates the swap chain when there is none yet, sets its
        /// transform and tells the host.
        fn ensure_swap_chain(&self, display: &AngleWin32EglDisplay) -> Result<ComPtr<IDXGISwapChain1>, String> {
            if let Some(existing) = self.inner.swap_chain.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
                return Ok((*existing.swap_chain).clone());
            }

            let swap_chain = self.create_swap_chain(display)?;

            let swap_chain_ptr = swap_chain.as_ptr() as isize;
            let swap_chain_unknown: &IUnknown = &swap_chain;
            let mut swap_chain3 = std::ptr::null_mut::<c_void>();
            // SAFETY: a live object, an identifier of this frame and the
            // place of one pointer.
            let qi_hr = unsafe { swap_chain_unknown.query_interface_raw(&IDXGI_SWAP_CHAIN3_GUID, &mut swap_chain3) };
            // SAFETY: a successful query answers with a counted reference.
            let swap_chain3 = (qi_hr == 0).then(|| unsafe { ComPtr::<IUnknown>::from_raw(swap_chain3.cast()) }).flatten();
            let Some(swap_chain3) = swap_chain3 else {
                return Err(format!("QI for IDXGISwapChain3 failed: HR=0x{qi_hr:08X}"));
            };

            // SAFETY (both): objects of DXGI, which are free-threaded.
            *self.inner.swap_chain.lock().unwrap_or_else(PoisonError::into_inner) = Some(unsafe {
                SwapChain { swap_chain: SharedCom::new(swap_chain.clone()), swap_chain3: SharedCom::new(swap_chain3) }
            });

            self.set_inverse_scale_transform((self.inner.get_scaling_func)())
                .map_err(|error| format!("IDXGISwapChain2::SetMatrixTransform failed: {error}"))?;

            (self.inner.set_swap_chain_callback)(swap_chain_ptr);
            Ok(swap_chain)
        }

        pub fn dispose_swap_chain(&self) {
            let swap_chain = self.inner.swap_chain.lock().unwrap_or_else(PoisonError::into_inner).take();
            drop(swap_chain);
        }
    }

    impl IPlatformRenderSurface for SwapChainGlSurface {
        fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
            if kind == TypeId::of::<dyn IGlPlatformSurface>() {
                let this: Rc<dyn IGlPlatformSurface> = Rc::new(SwapChainGlSurface { inner: self.inner.clone() });
                return Some(Rc::new(this));
            }
            None
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl EglGlPlatformSurfaceBase for SwapChainGlSurface {}

    impl IGlPlatformSurface for SwapChainGlSurface {
        /// # Panics
        /// Panics when `context` is not an EGL context of ANGLE (the
        /// failing casts of the reference) and when the swap chain cannot
        /// be created (the exceptions of the reference).
        fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
            let Some(egl_context) = context.as_any().downcast_ref::<EglContext>() else {
                panic!("Unable to cast the context to type 'EglContext'.");
            };
            let feature = egl_context.try_get_feature(TypeId::of::<AngleContextDisplay>());
            let Some(feature) = feature.as_ref().and_then(|feature| feature.downcast_ref::<AngleContextDisplay>()) else {
                panic!("Unable to cast the display of the context to type 'AngleWin32EglDisplay'.");
            };
            let Some(egl_context) = feature.context.upgrade() else {
                panic!("Cannot access a disposed object. Object name: 'EglContext'.");
            };

            let swap_chain = match self.ensure_swap_chain(&feature.display) {
                Ok(swap_chain) => swap_chain,
                Err(error) => panic!("{error}"),
            };

            Rc::new(SwapChainGlRenderTarget {
                base: EglPlatformSurfaceRenderTargetBase::new(egl_context),
                display: feature.display.clone(),
                swap_chain,
                owner: SwapChainGlSurface { inner: self.inner.clone() },
                render_texture: RefCell::new(None),
                surface: RefCell::new(None),
                last_size: Cell::new(PixelSize::default()),
                last_scaling: Cell::new(0.0),
            })
        }
    }

    pub(crate) struct SwapChainGlRenderTarget {
        base: EglPlatformSurfaceRenderTargetBase,
        /// The display of ANGLE of the context.
        display: Rc<AngleWin32EglDisplay>,
        swap_chain: ComPtr<IDXGISwapChain1>,
        owner: SwapChainGlSurface,

        render_texture: RefCell<Option<ComPtr<IUnknown>>>,
        surface: RefCell<Option<Rc<EglSurface>>>,
        last_size: Cell<PixelSize>,
        last_scaling: Cell<f64>,
    }

    impl SwapChainGlRenderTarget {
        const ID3D11_TEXTURE2D_GUID: Guid = crate::direct_x::ID3D11Texture2D::IID;

        fn release_surface(&self) {
            if let Some(surface) = self.surface.borrow_mut().take() {
                surface.dispose();
            }
        }

        fn begin_frame(&self, on_finish: Rc<dyn Fn()>) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, String> {
            let size = (self.owner.inner.get_size_func)();
            let scaling = (self.owner.inner.get_scaling_func)();

            if scaling != self.last_scaling.get() {
                self.owner
                    .set_inverse_scale_transform(scaling)
                    .map_err(|error| format!("IDXGISwapChain2::SetMatrixTransform failed: {error}"))?;
                self.last_scaling.set(scaling);
            }

            if size != self.last_size.get() {
                self.release_surface();
                *self.render_texture.borrow_mut() = None;

                self.swap_chain
                    .resize_buffers(
                        2,
                        u32::from(size.width as u16),
                        u32::from(size.height as u16),
                        DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
                        0,
                    )
                    .map_err(|error| format!("IDXGISwapChain::ResizeBuffers failed: {error}"))?;

                self.last_size.set(size);
            }

            let existing = self.render_texture.borrow().clone();
            let render_texture = match existing {
                Some(texture) => texture,
                None => {
                    self.release_surface();

                    let texture_guid = Self::ID3D11_TEXTURE2D_GUID;
                    // SAFETY: the identifier is a value of this frame; the
                    // result is the buffer as the interface asked for,
                    // with a reference this function owns.
                    let buffer = unsafe { self.swap_chain.get_buffer(0, &texture_guid) }
                        .map_err(|error| format!("IDXGISwapChain::GetBuffer failed: {error}"))?;
                    // SAFETY: as above.
                    let texture = unsafe { ComPtr::<IUnknown>::from_raw(buffer.cast()) }
                        .ok_or_else(|| "IDXGISwapChain::GetBuffer returned no buffer".to_owned())?;
                    *self.render_texture.borrow_mut() = Some(texture.clone());
                    texture
                }
            };

            let existing = self.surface.borrow().clone();
            let surface = match existing {
                Some(surface) => surface,
                None => {
                    let surface = self
                        .display
                        .wrap_direct3d11_texture_with_offset(render_texture.as_ptr() as isize, 0, 0, size.width, size.height)
                        .map_err(|error| error.to_string())?;
                    *self.surface.borrow_mut() = Some(surface.clone());
                    surface
                }
            };

            self.base.begin_draw(&surface, size, scaling, Some(on_finish), true, None, false).map_err(|error| error.to_string())
        }
    }

    impl EglPlatformSurfaceRenderTarget for SwapChainGlRenderTarget {
        fn base(&self) -> &EglPlatformSurfaceRenderTargetBase {
            &self.base
        }

        /// # Panics
        /// Panics when the frame cannot be begun (the exceptions of the
        /// reference).
        fn begin_draw_core(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
            let context_lock = self.base.context().ensure_current();
            let on_finish: Rc<dyn Fn()> = {
                let swap_chain = self.swap_chain.clone();
                let context_lock = context_lock.clone();
                Rc::new(move || {
                    swap_chain.present(1, 0);
                    context_lock.dispose();
                })
            };
            match self.begin_frame(on_finish) {
                Ok(session) => session,
                Err(error) => {
                    self.release_surface();
                    *self.render_texture.borrow_mut() = None;
                    context_lock.dispose();
                    panic!("{error}");
                }
            }
        }

        fn dispose(&self) {
            self.release_surface();
            *self.render_texture.borrow_mut() = None;
        }
    }

    impl IPlatformRenderSurfaceRenderTarget for SwapChainGlRenderTarget {
        fn state(&self) -> PlatformRenderTargetState {
            EglPlatformSurfaceRenderTarget::state(self)
        }
    }

    impl IGlPlatformSurfaceRenderTarget for SwapChainGlRenderTarget {
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
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the surface.
    use super::*;

    #[test]
    fn the_transform_of_the_swap_chain_undoes_the_scaling() {
        assert_eq!(None, inverse_scale_transform(0.0));
        assert_eq!(None, inverse_scale_transform(-1.0));
        assert_eq!(
            Some(DXGI_MATRIX_3X2_F { _11: 0.5, _12: 0.0, _21: 0.0, _22: 0.5, _31: 0.0, _32: 0.0 }),
            inverse_scale_transform(2.0)
        );
        assert_eq!(Some(DXGI_MATRIX_3X2_F { _11: 1.0, _22: 1.0, ..Default::default() }), inverse_scale_transform(1.0));
        assert_eq!(24, std::mem::size_of::<DXGI_MATRIX_3X2_F>());
    }

    #[test]
    fn set_matrix_transform_is_the_slot_of_the_system_header() {
        // dxgi1_3.h: the sixth method of IDXGISwapChain2, after the 29
        // of its bases.
        assert_eq!(34, SET_MATRIX_TRANSFORM_SLOT);
        assert_eq!(Guid::parse("94d99bdb-f1f8-4ab0-b236-7da0170edab1"), Some(IDXGI_SWAP_CHAIN3_GUID));
    }
}
