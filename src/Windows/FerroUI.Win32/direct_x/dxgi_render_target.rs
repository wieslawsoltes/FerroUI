//! The render target of a window in the DXGI swap chain mode: a flip-model
//! swap chain of the window on the Direct3D 11 device of the context of
//! ANGLE, whose back buffer is wrapped as an EGL surface for each frame.

use super::dxgi_connection::DxgiConnection;
use super::{
    IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, DXGI_ALPHA_MODE, DXGI_FORMAT, DXGI_MWA, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT,
};
use crate::interop::unmanaged_methods::{get_client_rect, RECT};
use crate::open_gl::angle::{AngleContextDisplay, AngleWin32EglDisplay};
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::{IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_microcom::{ComPtr, Guid, IUnknown, Interface};
use ferroui_opengl::egl::{
    EglContext, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase, EglSurface, IEglWindowGlPlatformSurfaceInfo,
};
use ferroui_opengl::surfaces::{IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// What a render target holds until it is disposed.
struct Chain {
    _dxgi_device: ComPtr<IDXGIDevice>,
    _dxgi_factory: ComPtr<IDXGIFactory2>,
    swap_chain: ComPtr<IDXGISwapChain1>,
}

pub(crate) struct DxgiRenderTarget {
    base: EglPlatformSurfaceRenderTargetBase,
    window: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    /// Held as in the reference, which does not use it either.
    _connection: Arc<DxgiConnection>,
    /// The display of ANGLE of the context (`(AngleWin32EglDisplay)context.Display`).
    display: Rc<AngleWin32EglDisplay>,
    /// `None` once disposed.
    chain: RefCell<Option<Chain>>,
    flags_used: u32,

    render_texture: RefCell<Option<ComPtr<IUnknown>>>,
    client_rect: Cell<RECT>,
    surface: RefCell<Option<Rc<EglSurface>>>,
}

impl DxgiRenderTarget {
    // DXGI_FORMAT_B8G8R8A8_UNORM is target texture format as per ANGLE documentation

    pub const DXGI_USAGE_RENDER_TARGET_OUTPUT: u32 = 0x0000_0020;
    const ID3D11_TEXTURE2D_GUID: Guid = super::ID3D11Texture2D::IID;

    /// The swap chain of the window on the device of the context. A
    /// failure is the exception of the reference, as its message.
    ///
    /// The reference casts the display of the context to the display of
    /// ANGLE; a context of the port has the wrapper as a feature
    /// (`D3D11AngleWin32PlatformGraphics`), with its own shared handle.
    pub fn new(
        window: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
        context: &EglContext,
        connection: Arc<DxgiConnection>,
    ) -> Result<DxgiRenderTarget, String> {
        let no_display = || "Unable to cast the display of the context to type 'AngleWin32EglDisplay'.".to_owned();
        let feature = context.try_get_feature(TypeId::of::<AngleContextDisplay>()).ok_or_else(no_display)?;
        let feature = feature.downcast_ref::<AngleContextDisplay>().ok_or_else(no_display)?;
        let display = feature.display.clone();
        let context = feature.context.upgrade().ok_or_else(|| "Cannot access a disposed object. Object name: 'EglContext'.".to_owned())?;

        // the D3D device is expected to at least be an ID3D11Device
        let device = display.get_direct3d_device().map_err(|error| error.to_string())?;
        // SAFETY: the display answers with a pointer to its device of
        // Direct3D, which lives as long as the display; a reference is
        // taken for the time of the query below.
        let pdevice = unsafe { ComPtr::<IUnknown>::from_raw_add_ref(device as *mut IUnknown) }
            .ok_or_else(|| "The display has no Direct3D device".to_owned())?;

        let failed = |call: &str| {
            let call = call.to_owned();
            move |error: ferroui_microcom::HResult| format!("{call} failed: {error}")
        };
        let dxgi_device = pdevice.cast::<IDXGIDevice>().map_err(failed("QueryInterface(IDXGIDevice)"))?;

        // only needing the adapter pointer to ask it for the IDXGI Factory
        let dxgi_factory = {
            let adapter_pointer = dxgi_device
                .get_adapter()
                .map_err(failed("IDXGIDevice::GetAdapter"))?
                .ok_or_else(|| "IDXGIDevice::GetAdapter returned no adapter".to_owned())?;
            let factory_guid = IDXGIFactory2::IID;
            // SAFETY: the identifier is a value of this frame; the result
            // is the parent as the interface asked for, with a reference
            // this function owns.
            let parent = unsafe { adapter_pointer.get_parent(&factory_guid) }.map_err(failed("IDXGIAdapter::GetParent"))?;
            // SAFETY: as above.
            unsafe { ComPtr::<IDXGIFactory2>::from_raw(parent.cast()) }
                .ok_or_else(|| "IDXGIAdapter::GetParent returned no factory".to_owned())?
        };

        let mut dxgi_swap_chain_desc = DXGI_SWAP_CHAIN_DESC1::default();

        // standard swap chain really.
        dxgi_swap_chain_desc.format = DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM;
        dxgi_swap_chain_desc.sample_desc.count = 1;
        dxgi_swap_chain_desc.sample_desc.quality = 0;
        dxgi_swap_chain_desc.buffer_usage = Self::DXGI_USAGE_RENDER_TARGET_OUTPUT;
        dxgi_swap_chain_desc.alpha_mode = DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_IGNORE;
        dxgi_swap_chain_desc.width = window.size().width as u32;
        dxgi_swap_chain_desc.height = window.size().height as u32;
        dxgi_swap_chain_desc.buffer_count = 2;
        dxgi_swap_chain_desc.swap_effect = DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_DISCARD;

        // okay I know this looks bad, but we're hitting our render-calls by awaiting via dxgi
        // this is done in the DxgiConnection itself
        let flags_used = DXGI_SWAP_CHAIN_FLAG::DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING.0 as u32;
        dxgi_swap_chain_desc.flags = flags_used;

        let device_unknown: &IUnknown = &dxgi_device;
        // SAFETY: the description is a structure of this frame that lives
        // through the call; no full-screen description and no output.
        let swap_chain = unsafe {
            dxgi_factory.create_swap_chain_for_hwnd(
                Some(device_unknown),
                window.handle(),
                &mut dxgi_swap_chain_desc,
                std::ptr::null_mut(),
                None,
            )
        }
        .map_err(failed("IDXGIFactory2::CreateSwapChainForHwnd"))?
        .ok_or_else(|| "IDXGIFactory2::CreateSwapChainForHwnd returned no swap chain".to_owned())?;

        dxgi_factory
            .make_window_association(
                window.handle(),
                (DXGI_MWA::DXGI_MWA_NO_ALT_ENTER.0 | DXGI_MWA::DXGI_MWA_NO_PRINT_SCREEN.0) as u32,
            )
            .map_err(failed("IDXGIFactory::MakeWindowAssociation"))?;

        let client_rect = get_client_rect(window.handle());

        Ok(DxgiRenderTarget {
            base: EglPlatformSurfaceRenderTargetBase::new(context),
            window,
            _connection: connection,
            display,
            chain: RefCell::new(Some(Chain { _dxgi_device: dxgi_device, _dxgi_factory: dxgi_factory, swap_chain })),
            flags_used,
            render_texture: RefCell::new(None),
            client_rect: Cell::new(client_rect),
            surface: RefCell::new(None),
        })
    }

    fn release_surface(&self) {
        if let Some(surface) = self.surface.borrow_mut().take() {
            surface.dispose();
        }
    }

    /// The frame: the back buffer of the swap chain as an EGL surface,
    /// presented when the session is disposed.
    fn begin_frame(
        &self,
        swap_chain: &ComPtr<IDXGISwapChain1>,
        on_finish: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, String> {
        let p_client_rect = get_client_rect(self.window.handle());
        if !Self::rects_equal(&p_client_rect, &self.client_rect.get()) {
            // we gotta resize
            self.client_rect.set(p_client_rect);

            if self.render_texture.borrow().is_some() {
                self.release_surface();
                *self.render_texture.borrow_mut() = None;
            }

            swap_chain
                .resize_buffers(
                    2,
                    u32::from((p_client_rect.right - p_client_rect.left) as u16),
                    u32::from((p_client_rect.bottom - p_client_rect.top) as u16),
                    DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
                    u32::from(self.flags_used as u16),
                )
                .map_err(|error| format!("IDXGISwapChain::ResizeBuffers failed: {error}"))?;
        }

        let size = self.window.size();

        // Get swapchain texture here
        let existing = self.render_texture.borrow().clone();
        let texture = match existing {
            Some(texture) => texture,
            None => {
                self.release_surface();

                let texture_guid = Self::ID3D11_TEXTURE2D_GUID;
                // SAFETY: the identifier is a value of this frame; the
                // result is the buffer as the interface asked for, with a
                // reference this function owns.
                let buffer = unsafe { swap_chain.get_buffer(0, &texture_guid) }
                    .map_err(|error| format!("IDXGISwapChain::GetBuffer failed: {error}"))?;
                // SAFETY: as above.
                unsafe { ComPtr::<IUnknown>::from_raw(buffer.cast()) }
                    .ok_or_else(|| "IDXGISwapChain::GetBuffer returned no buffer".to_owned())?
            }
        };
        *self.render_texture.borrow_mut() = Some(texture.clone());

        let existing = self.surface.borrow().clone();
        let surface = match existing {
            Some(surface) => surface,
            None => {
                // I also have to get the pointer to this texture directly
                let surface = self
                    .display
                    .wrap_direct3d11_texture_with_offset(texture.as_ptr() as isize, 0, 0, size.width, size.height)
                    .map_err(|error| error.to_string())?;
                *self.surface.borrow_mut() = Some(surface.clone());
                surface
            }
        };

        self.base
            .begin_draw(&surface, self.window.size(), self.window.scaling(), Some(on_finish), true, None, false)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn rects_equal(l: &RECT, r: &RECT) -> bool {
        (l.left == r.left) && (l.top == r.top) && (l.right == r.right) && (l.bottom == r.bottom)
    }
}

impl EglPlatformSurfaceRenderTarget for DxgiRenderTarget {
    fn base(&self) -> &EglPlatformSurfaceRenderTargetBase {
        &self.base
    }

    /// # Panics
    /// Panics when the target was disposed ("No chain to draw on") and
    /// when the frame cannot be begun (the exceptions of the reference).
    fn begin_draw_core(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        // TODO: use expectedPixelSize
        let Some(swap_chain) = self.chain.borrow().as_ref().map(|chain| chain.swap_chain.clone()) else {
            panic!("No chain to draw on");
        };

        let context_lock = self.base.context().ensure_current();
        let on_finish: Rc<dyn Fn()> = {
            let swap_chain = swap_chain.clone();
            let context_lock = context_lock.clone();
            Rc::new(move || {
                swap_chain.present(0, 0);
                context_lock.dispose();
            })
        };
        match self.begin_frame(&swap_chain, on_finish) {
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
        *self.chain.borrow_mut() = None;
        self.release_surface();
        *self.render_texture.borrow_mut() = None;
    }
}

impl IPlatformRenderSurfaceRenderTarget for DxgiRenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        EglPlatformSurfaceRenderTarget::state(self)
    }
}

impl IGlPlatformSurfaceRenderTarget for DxgiRenderTarget {
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
    // Not from upstream: the reference has no tests of the render target.
    use super::*;

    #[test]
    fn rectangles_are_compared_by_their_four_edges() {
        let rect = RECT { left: 0, top: 0, right: 640, bottom: 400 };
        assert!(DxgiRenderTarget::rects_equal(&rect, &rect.clone()));
        assert!(!DxgiRenderTarget::rects_equal(&rect, &RECT { right: 641, ..rect }));
        assert!(!DxgiRenderTarget::rects_equal(&rect, &RECT { bottom: 399, ..rect }));
        assert!(!DxgiRenderTarget::rects_equal(&rect, &RECT { left: 1, ..rect }));
        assert!(!DxgiRenderTarget::rects_equal(&rect, &RECT { top: -1, ..rect }));
        assert_eq!(0x20, DxgiRenderTarget::DXGI_USAGE_RENDER_TARGET_OUTPUT);
    }
}
