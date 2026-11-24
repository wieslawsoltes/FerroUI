//! A render target that draws into a DXGI flip-model swap chain made for
//! composition (`CreateSwapChainForComposition`) and shown as the content
//! of a visual: the window of the DirectComposition or the
//! Windows.UI.Composition mode with `Win32PlatformOptions::use_composition_swap_chain`,
//! or the swap chain panel of the WinUI integration (upstream
//! `CompositionSwapchainRenderTarget`, db0a9a9cdb).
//!
//! Unlike the drawing surfaces of the composition modes, a swap chain lets
//! the desktop window manager use its optimized presentation modes
//! (independent flip) for a window that fills the screen.

use super::{DXGI_ALPHA_MODE, DXGI_FORMAT, DXGI_SAMPLE_DESC, DXGI_SCALING, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT};
use crate::platform_constants::{PlatformConstants, Version};
use ferroui_base::PixelSize;
use ferroui_microcom::Guid;

/// `DXGI_USAGE_RENDER_TARGET_OUTPUT`.
const DXGI_USAGE_RENDER_TARGET_OUTPUT: u32 = 0x0000_0020;

/// The number of buffers of the swap chain.
const BUFFER_COUNT: u32 = 2;

/// `IDXGISwapChain3`. The query for `IDXGISwapChain2` fails on some
/// Windows builds although the third and fourth versions answer; the third
/// derives from the second and has `SetMatrixTransform` at the same slot.
pub const IDXGI_SWAP_CHAIN3_GUID: Guid =
    Guid::new(0x94d9_9bdb, 0xf1f8, 0x4ab0, [0xb2, 0x36, 0x7d, 0xa0, 0x17, 0x0e, 0xda, 0xb1]);

/// The slot of `IDXGISwapChain2::SetMatrixTransform` in its vtable:
/// `IUnknown` (3), `IDXGIObject` (4), `IDXGIDeviceSubObject` (1),
/// `IDXGISwapChain` (10), `IDXGISwapChain1` (11), then `SetSourceSize`,
/// `GetSourceSize`, `SetMaximumFrameLatency`, `GetMaximumFrameLatency` and
/// `GetFrameLatencyWaitableObject` (5).
pub const SET_MATRIX_TRANSFORM_SLOT: usize = 3 + 4 + 1 + 10 + 11 + 5;

/// `DXGI_MATRIX_3X2_F`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(non_camel_case_types)]
pub struct DXGI_MATRIX_3X2_F {
    pub _11: f32,
    pub _12: f32,
    pub _21: f32,
    pub _22: f32,
    pub _31: f32,
    pub _32: f32,
}

/// The transform that undoes the scaling of a host that composes the swap
/// chain in device-independent units (a swap chain panel): the swap chain
/// has the size in pixels. `None` for a scaling that is not positive.
pub fn inverse_scale_transform(scaling: f64) -> Option<DXGI_MATRIX_3X2_F> {
    if scaling <= 0.0 {
        return None;
    }

    Some(DXGI_MATRIX_3X2_F { _11: 1.0 / scaling as f32, _22: 1.0 / scaling as f32, ..Default::default() })
}

/// The size of the buffers of a frame: a swap chain for composition, unlike
/// one of a window, needs non-zero dimensions.
pub(crate) fn swap_chain_size(size: PixelSize) -> PixelSize {
    PixelSize::new(size.width.max(1), size.height.max(1))
}

/// The description of the swap chain of a frame of `size`.
///
/// Premultiplied alpha costs the window manager more and keeps it from the
/// optimized presentation modes, so it is used only when the window is
/// transparent. `FLIP_DISCARD` does not exist before Windows 10.
pub(crate) fn swap_chain_desc(size: PixelSize, is_transparency: bool, windows_version: Version) -> DXGI_SWAP_CHAIN_DESC1 {
    DXGI_SWAP_CHAIN_DESC1 {
        width: size.width as u32,
        height: size.height as u32,
        format: DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
        sample_desc: DXGI_SAMPLE_DESC { count: 1, quality: 0 },
        buffer_usage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        buffer_count: BUFFER_COUNT,
        // The only scaling a swap chain for composition takes: the visual decides how its
        // content is shown (a DirectComposition visual at its own size; the Windows.UI
        // composition brush is set not to stretch, section 6.6.1 of docs/porting/win32-platform.md).
        scaling: DXGI_SCALING::DXGI_SCALING_STRETCH,
        swap_effect: if windows_version >= PlatformConstants::WINDOWS10 {
            DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_DISCARD
        } else {
            DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL
        },
        alpha_mode: if is_transparency {
            DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_PREMULTIPLIED
        } else {
            DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_IGNORE
        },
        flags: 0,
        ..Default::default()
    }
}

/// What a frame does with the swap chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwapChainUpdate {
    /// The swap chain fits the frame.
    Keep,
    /// The buffers are resized (`ResizeBuffers`).
    Resize,
    /// A new swap chain is made and attached: there is none, or the alpha
    /// mode changed, which `ResizeBuffers` cannot change.
    Create,
}

/// What a frame of `size` and transparency does with the swap chain that
/// exists (`Some((size, is_transparency))`) or does not.
pub(crate) fn swap_chain_update(existing: Option<(PixelSize, bool)>, size: PixelSize, is_transparency: bool) -> SwapChainUpdate {
    match existing {
        None => SwapChainUpdate::Create,
        Some((_, transparency)) if transparency != is_transparency => SwapChainUpdate::Create,
        Some((existing_size, _)) if existing_size != size => SwapChainUpdate::Resize,
        Some(_) => SwapChainUpdate::Keep,
    }
}

#[cfg(windows)]
pub use imp::{CompositionSwapchainSurface, ISwapchainVisualHost};
#[cfg(windows)]
pub(crate) use imp::CompositionSwapchainRenderTarget;

#[cfg(windows)]
mod imp {
    use super::{swap_chain_desc, swap_chain_size, swap_chain_update, SwapChainUpdate, BUFFER_COUNT};
    use crate::direct_x::{
        IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, IDirect3D11TexturePlatformSurface,
        IDirect3D11TexturePlatformSurface2, IDirect3D11TextureRenderTarget, IDirect3D11TextureRenderTarget2,
        IDirect3D11TextureRenderTargetRenderSession, ID3D11Texture2D, DXGI_FORMAT,
    };
    use crate::win32_platform::Win32Platform;
    use crate::win32_top_level_scene_info::Win32TopLevelSceneInfo;
    use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
    use ferroui_base::platform::{
        IPlatformGraphicsContext, PlatformRenderTargetState, PlatformThemeVariant, RenderTargetSceneInfo,
    };
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::{PixelPoint, PixelSize, RenderTargetCorruptedError};
    use ferroui_microcom::{ComPtr, HResult, IUnknown, Interface};
    use std::any::{Any, TypeId};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::Arc;

    /// A composed window that shows a DXGI swap chain as its content. Its
    /// members are called on the thread that renders.
    pub trait ISwapchainVisualHost: Send + Sync {
        /// The size of the window in pixels: the size of a frame begun
        /// without a scene.
        fn window_size(&self) -> PixelSize;

        /// The scaling of the window, for a frame begun without a scene.
        fn window_scaling(&self) -> f64;

        /// Begins the changes of a frame (enters the lock of the composition
        /// of the host); [`end_transaction`](Self::end_transaction) commits
        /// them. Every begin is ended once.
        fn begin_transaction(&self);

        fn end_transaction(&self);

        /// Makes the swap chain the content of the visual of the window.
        /// Called inside the transaction, only when the swap chain is made
        /// or made again.
        fn set_swapchain_content(&self, swap_chain: &ComPtr<IDXGISwapChain1>) -> Result<(), HResult>;

        /// The frame has `size`, inside the transaction.
        fn resize_if_needed(&self, size: PixelSize) -> Result<(), HResult>;

        /// The backdrop effects of the frame, inside the transaction.
        fn apply_effects(
            &self,
            transparency_level: CompositionTransparencyLevel,
            theme_variant: PlatformThemeVariant,
        ) -> Result<(), HResult>;
    }

    /// A render surface drawn into a swap chain for composition that `host`
    /// shows (the surface the swap chain panel of the WinUI integration
    /// gives its top level).
    pub struct CompositionSwapchainSurface {
        host: Arc<dyn ISwapchainVisualHost>,
    }

    impl CompositionSwapchainSurface {
        pub fn new(host: Arc<dyn ISwapchainVisualHost>) -> Arc<CompositionSwapchainSurface> {
            Arc::new(CompositionSwapchainSurface { host })
        }

        fn create_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<CompositionSwapchainRenderTarget>, HResult> {
            CompositionSwapchainRenderTarget::new(context.clone(), d3d_device, self.host.clone())
        }
    }

    impl IPlatformRenderSurface for CompositionSwapchainSurface {
        fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
            let view = || Rc::new(CompositionSwapchainSurface { host: self.host.clone() });
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

    impl IDirect3D11TexturePlatformSurface for CompositionSwapchainSurface {
        fn create_render_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget>, HResult> {
            Ok(self.create_target(context, d3d_device)?)
        }
    }

    impl IDirect3D11TexturePlatformSurface2 for CompositionSwapchainSurface {
        fn create_render_target(
            &self,
            context: &Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget2>, HResult> {
            Ok(self.create_target(context, d3d_device)?)
        }
    }

    /// What the render target holds until it is disposed.
    struct Device {
        _d3d_device: ComPtr<IUnknown>,
        dxgi_device: ComPtr<IDXGIDevice>,
        dxgi_factory: ComPtr<IDXGIFactory2>,
    }

    /// The swap chain, with the size of its buffers and whether its alpha is
    /// premultiplied.
    struct SwapChain {
        swap_chain: ComPtr<IDXGISwapChain1>,
        size: PixelSize,
        is_transparency: bool,
    }

    pub(crate) struct CompositionSwapchainRenderTarget {
        context: Rc<dyn IPlatformGraphicsContext>,
        host: Arc<dyn ISwapchainVisualHost>,
        /// `None` once disposed.
        device: RefCell<Option<Device>>,
        swap_chain: RefCell<Option<SwapChain>>,
        lost: Cell<bool>,
    }

    impl CompositionSwapchainRenderTarget {
        pub(crate) fn new(
            context: Rc<dyn IPlatformGraphicsContext>,
            d3d_device: isize,
            host: Arc<dyn ISwapchainVisualHost>,
        ) -> Result<Rc<CompositionSwapchainRenderTarget>, HResult> {
            // SAFETY: the caller of the surface contract passes a pointer to the device of
            // Direct3D 11 of its display, which lives through the call; the render target takes
            // a reference of its own.
            let d3d_device =
                unsafe { ComPtr::<IUnknown>::from_raw_add_ref(d3d_device as *mut IUnknown) }.ok_or(HResult::POINTER)?;
            let dxgi_device = d3d_device.cast::<IDXGIDevice>()?;
            let adapter = dxgi_device.get_adapter()?.ok_or(HResult::POINTER)?;
            let factory_guid = IDXGIFactory2::IID;
            // SAFETY: the identifier is a value of this frame; the result is the parent as the
            // interface asked for, with a reference this function owns.
            let parent = unsafe { adapter.get_parent(&factory_guid) }?;
            // SAFETY: as above.
            let dxgi_factory = unsafe { ComPtr::<IDXGIFactory2>::from_raw(parent.cast()) }.ok_or(HResult::POINTER)?;
            // A failure above releases what was created before it, as the reference does in its
            // `catch`.
            Ok(Rc::new(CompositionSwapchainRenderTarget {
                context,
                host,
                device: RefCell::new(Some(Device { _d3d_device: d3d_device, dxgi_device, dxgi_factory })),
                swap_chain: RefCell::new(None),
                lost: Cell::new(false),
            }))
        }

        fn dispose_target(&self) {
            *self.swap_chain.borrow_mut() = None;
            *self.device.borrow_mut() = None;
        }

        fn current_state(&self) -> PlatformRenderTargetState {
            if self.context.is_lost() || self.lost.get() {
                PlatformRenderTargetState::CORRUPTED
            } else {
                PlatformRenderTargetState::READY
            }
        }

        /// Makes the swap chain of a frame and attaches it to the visual.
        fn create_swap_chain(&self, size: PixelSize, is_transparency: bool) -> Result<SwapChain, HResult> {
            let device = self.device.borrow();
            let device = device.as_ref().ok_or(HResult::OBJECTDISPOSED)?;
            let mut desc = swap_chain_desc(size, is_transparency, Win32Platform::windows_version());
            let device_unknown: &IUnknown = &device.dxgi_device;
            // SAFETY: the description is a structure of this frame that lives through the call;
            // no output.
            let swap_chain = unsafe {
                device.dxgi_factory.create_swap_chain_for_composition(Some(device_unknown), &mut desc, None)
            }?
            .ok_or(HResult::POINTER)?;
            // A failure to attach drops the swap chain.
            self.host.set_swapchain_content(&swap_chain)?;
            Ok(SwapChain { swap_chain, size, is_transparency })
        }

        /// The swap chain of the frame, made, resized or as it is.
        fn prepare_swap_chain(&self, size: PixelSize, is_transparency: bool) -> Result<ComPtr<IDXGISwapChain1>, HResult> {
            let existing = self.swap_chain.borrow().as_ref().map(|chain| (chain.size, chain.is_transparency));
            match swap_chain_update(existing, size, is_transparency) {
                SwapChainUpdate::Create => {
                    // The alpha mode cannot be changed by `ResizeBuffers`: the swap chain is made
                    // again when the transparency level changes it.
                    *self.swap_chain.borrow_mut() = None;
                    let created = self.create_swap_chain(size, is_transparency)?;
                    let swap_chain = created.swap_chain.clone();
                    *self.swap_chain.borrow_mut() = Some(created);
                    Ok(swap_chain)
                }
                SwapChainUpdate::Resize => {
                    let mut chain = self.swap_chain.borrow_mut();
                    let chain = chain.as_mut().ok_or(HResult::POINTER)?;
                    // No reference to a back buffer is left: the session of the previous frame
                    // released its texture before it presented.
                    chain.swap_chain.resize_buffers(
                        BUFFER_COUNT,
                        size.width as u32,
                        size.height as u32,
                        DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
                        0,
                    )?;
                    chain.size = size;
                    Ok(chain.swap_chain.clone())
                }
                SwapChainUpdate::Keep => {
                    let chain = self.swap_chain.borrow();
                    Ok(chain.as_ref().ok_or(HResult::POINTER)?.swap_chain.clone())
                }
            }
        }

        fn begin_draw_scene(
            &self,
            scene_info: &RenderTargetSceneInfo,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedError> {
            if self.current_state().is_corrupted {
                return Err(RenderTargetCorruptedError::new());
            }
            // The transaction ends when the guard is dropped: at the end of the session, or here
            // when the frame cannot be begun.
            self.host.begin_transaction();
            let transaction = TransactionGuard(self.host.clone());

            let is_transparency = scene_info.transparency_level != CompositionTransparencyLevel::None;
            let size = swap_chain_size(scene_info.size);
            let theme_variant = scene_info
                .platform_specific_scene_info
                .as_ref()
                .and_then(|info| info.downcast_ref::<Win32TopLevelSceneInfo>())
                .map_or(PlatformThemeVariant::Light, |info| info.theme_variant);

            let frame = || -> Result<(ComPtr<IDXGISwapChain1>, ComPtr<IUnknown>), HResult> {
                let swap_chain = self.prepare_swap_chain(size, is_transparency)?;
                self.host.resize_if_needed(size)?;
                self.host.apply_effects(scene_info.transparency_level, theme_variant)?;
                let iid = ID3D11Texture2D::IID;
                // SAFETY: the identifier is a value of this frame; the result is the buffer as
                // the interface asked for, with a reference this function owns.
                let buffer = unsafe { swap_chain.get_buffer(0, &iid) }?;
                // SAFETY: as above.
                let texture = unsafe { ComPtr::<IUnknown>::from_raw(buffer.cast()) }.ok_or(HResult::POINTER)?;
                Ok((swap_chain, texture))
            };
            let (swap_chain, texture) = frame().map_err(|error| {
                self.lost.set(true);
                RenderTargetCorruptedError::new_with_inner_exception(Rc::new(error))
            })?;

            Ok(Rc::new(Session {
                texture_pointer: Cell::new(texture.as_ptr() as isize),
                state: RefCell::new(Some(SessionState { swap_chain, texture, _transaction: transaction })),
                size,
                scaling: scene_info.scaling,
            }))
        }
    }

    impl IPlatformRenderSurfaceRenderTarget for CompositionSwapchainRenderTarget {
        fn state(&self) -> PlatformRenderTargetState {
            self.current_state()
        }
    }

    impl IDirect3D11TextureRenderTarget for CompositionSwapchainRenderTarget {
        fn begin_draw(&self) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedError> {
            let fallback_scene_info = RenderTargetSceneInfo::new(
                self.host.window_size(),
                self.host.window_scaling(),
                CompositionTransparencyLevel::None,
            );
            self.begin_draw_scene(&fallback_scene_info)
        }

        fn dispose(&self) {
            self.dispose_target();
        }
    }

    impl IDirect3D11TextureRenderTarget2 for CompositionSwapchainRenderTarget {
        fn begin_draw(
            &self,
            scene_info: &RenderTargetSceneInfo,
        ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedError> {
            self.begin_draw_scene(scene_info)
        }

        fn dispose(&self) {
            self.dispose_target();
        }
    }

    /// Ends the transaction of the host when dropped.
    struct TransactionGuard(Arc<dyn ISwapchainVisualHost>);

    impl Drop for TransactionGuard {
        fn drop(&mut self) {
            self.0.end_transaction();
        }
    }

    struct SessionState {
        swap_chain: ComPtr<IDXGISwapChain1>,
        texture: ComPtr<IUnknown>,
        _transaction: TransactionGuard,
    }

    struct Session {
        /// `None` once disposed.
        state: RefCell<Option<SessionState>>,
        texture_pointer: Cell<isize>,
        size: PixelSize,
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
            PixelPoint::default()
        }

        fn scaling(&self) -> f64 {
            self.scaling
        }

        fn dispose(&self) {
            let Some(SessionState { swap_chain, texture, _transaction }) = self.state.borrow_mut().take() else {
                return;
            };
            self.texture_pointer.set(0);
            // The reference to the back buffer goes before the next `ResizeBuffers` may run.
            // The frame is presented before the transaction commits, so a swap chain that was
            // just attached has a frame when it becomes visible. No tearing: `Present(0, 0)`.
            drop(texture);
            let _ = swap_chain.present(0, 0);
            drop(swap_chain);
            drop(_transaction);
        }
    }

    impl Drop for Session {
        fn drop(&mut self) {
            // A session that is dropped without being disposed still presents its frame and ends
            // the transaction.
            IDirect3D11TextureRenderTargetRenderSession::dispose(self);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the render target.
    use super::*;

    #[test]
    fn the_swap_chain_is_made_again_only_when_the_alpha_mode_changes() {
        let size = PixelSize::new(640, 400);
        assert_eq!(swap_chain_update(None, size, false), SwapChainUpdate::Create);
        assert_eq!(swap_chain_update(Some((size, false)), size, false), SwapChainUpdate::Keep);
        assert_eq!(swap_chain_update(Some((size, false)), PixelSize::new(641, 400), false), SwapChainUpdate::Resize);
        assert_eq!(swap_chain_update(Some((size, false)), size, true), SwapChainUpdate::Create);
        assert_eq!(swap_chain_update(Some((size, true)), PixelSize::new(10, 10), false), SwapChainUpdate::Create);
    }

    #[test]
    fn a_swap_chain_for_composition_has_buffers_of_at_least_one_pixel() {
        assert_eq!(swap_chain_size(PixelSize::new(0, 0)), PixelSize::new(1, 1));
        assert_eq!(swap_chain_size(PixelSize::new(0, 300)), PixelSize::new(1, 300));
        assert_eq!(swap_chain_size(PixelSize::new(800, 600)), PixelSize::new(800, 600));
    }

    #[test]
    fn the_description_premultiplies_only_a_transparent_window_and_flips_by_version() {
        let size = PixelSize::new(800, 600);
        let windows8_1 = Version::new(6, 3);
        let opaque = swap_chain_desc(size, false, PlatformConstants::WINDOWS10);
        assert_eq!((opaque.width, opaque.height), (800, 600));
        assert_eq!(opaque.alpha_mode, DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_IGNORE);
        assert_eq!(opaque.swap_effect, DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_DISCARD);
        // `DXGI_FORMAT` is not `Debug`.
        assert!(opaque.format == DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM);
        assert_eq!((opaque.buffer_count, opaque.buffer_usage, opaque.flags), (2, 0x20, 0));
        assert_eq!(opaque.scaling, DXGI_SCALING::DXGI_SCALING_STRETCH);

        let transparent = swap_chain_desc(size, true, windows8_1);
        assert_eq!(transparent.alpha_mode, DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_PREMULTIPLIED);
        assert_eq!(transparent.swap_effect, DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL);
    }

    #[test]
    fn the_inverse_scale_transform_undoes_a_positive_scaling() {
        assert_eq!(inverse_scale_transform(0.0), None);
        assert_eq!(inverse_scale_transform(-1.0), None);
        let transform = inverse_scale_transform(2.0).unwrap();
        assert_eq!((transform._11, transform._22), (0.5, 0.5));
        assert_eq!((transform._12, transform._21, transform._31, transform._32), (0.0, 0.0, 0.0, 0.0));
        assert_eq!(SET_MATRIX_TRANSFORM_SLOT, 34);
    }
}
