//! Port of `D3DDemo/D3D11Swapchain.cs` (Windows): the textures the Direct3D demo draws its
//! frames into, shared with the compositor by their global shared handles and synchronized by
//! their keyed mutexes.

use super::d3d11::{
    required, throw_h_result, ID3D11Device, ID3D11Texture2D, IDXGIKeyedMutex, IDXGIResource, D3D11_BIND_RENDER_TARGET,
    D3D11_BIND_SHADER_RESOURCE, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
};
use crate::task_support::{join, present_status, start, ServerJobTaskFuture};
use ferroui_base::platform::{
    KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
    PlatformHandle,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, ICompositionGpuInterop, ICompositionImportedGpuImage, ServerJobTask,
};
use ferroui_base::rendering::{ISwapchainImage, SwapchainBase, SwapchainImageFactory, SwapchainImagePresentStatus};
use ferroui_base::threading::JoinHandle;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, IUnknown};
use std::cell::RefCell;
use std::rc::Rc;

type Disposals = Rc<RefCell<Vec<JoinHandle<()>>>>;

pub struct D3D11Swapchain {
    base: SwapchainBase<D3D11SwapchainImage>,
    device: ComPtr<ID3D11Device>,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    disposals: Disposals,
}

impl D3D11Swapchain {
    pub fn new(device: &ComPtr<ID3D11Device>, interop: &Rc<dyn ICompositionGpuInterop>, target: &CompositionDrawingSurface) -> Self {
        Self {
            base: SwapchainBase::new(),
            device: device.clone(),
            interop: interop.clone(),
            target: target.clone(),
            disposals: Rc::default(),
        }
    }

    /// Starts a frame of `size`: the session, whose disposal presents the frame, and the render
    /// target view of the texture to draw it into.
    pub fn begin_draw(&self, size: PixelSize) -> (Rc<dyn IDisposable>, ComPtr<IUnknown>) {
        let (rv, image) = self.base.begin_draw_core(self, size);
        let view = image.render_target_view.clone();
        (rv, view)
    }

    /// Releases the textures once the compositor is done with them.
    pub fn dispose_async(&self) -> JoinHandle<()> {
        self.base.dispose();
        let disposals = std::mem::take(&mut *self.disposals.borrow_mut());
        start(async move {
            for disposal in disposals {
                let _ = join(disposal).await;
            }
        })
    }
}

impl SwapchainImageFactory<D3D11SwapchainImage> for D3D11Swapchain {
    /// # Panics
    /// Panics when the texture cannot be created (the exception upstream, which ends the
    /// drawing of the frame).
    fn create_image(&self, size: PixelSize) -> Rc<D3D11SwapchainImage> {
        match D3D11SwapchainImage::new(&self.device, size, &self.interop, &self.target, &self.disposals) {
            Ok(image) => Rc::new(image),
            Err(error) => panic!("{error}"),
        }
    }
}

pub struct D3D11SwapchainImage {
    size: PixelSize,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    texture: ComPtr<ID3D11Texture2D>,
    mutex: ComPtr<IDXGIKeyedMutex>,
    render_target_view: ComPtr<IUnknown>,
    handle: isize,
    properties: PlatformGraphicsExternalImageProperties,
    imported: RefCell<Option<Rc<dyn ICompositionImportedGpuImage>>>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
    disposals: Disposals,
}

impl D3D11SwapchainImage {
    fn new(
        device: &ComPtr<ID3D11Device>,
        size: PixelSize,
        interop: &Rc<dyn ICompositionGpuInterop>,
        target: &CompositionDrawingSurface,
        disposals: &Disposals,
    ) -> Result<Self, String> {
        let mut texture_desc = D3D11_TEXTURE2D_DESC {
            format: DXGI_FORMAT_R8G8B8A8_UNORM,
            width: size.width as u32,
            height: size.height as u32,
            array_size: 1,
            mip_levels: 1,
            sample_desc: DXGI_SAMPLE_DESC::new(1, 0),
            usage: D3D11_USAGE_DEFAULT,
            bind_flags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
            cpu_access_flags: 0,
            misc_flags: D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX,
        };
        // SAFETY: the description is alive for the call, and there is no initial data.
        let texture = required(
            throw_h_result(unsafe { device.create_texture2d(&mut texture_desc, std::ptr::null_mut()) })?,
            "ID3D11Device::CreateTexture2D",
        )?;

        let mutex = throw_h_result(texture.cast::<IDXGIKeyedMutex>())?;
        let handle = {
            let res = throw_h_result(texture.cast::<IDXGIResource>())?;
            throw_h_result(res.get_shared_handle())?
        };

        let properties = PlatformGraphicsExternalImageProperties {
            width: size.width,
            height: size.height,
            format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
            ..Default::default()
        };

        // SAFETY: the texture is a resource of the device; no description: a view of the whole
        // texture.
        let render_target_view = required(
            throw_h_result(unsafe {
                device.create_render_target_view(Some(&**texture), std::ptr::null_mut())
            })?,
            "ID3D11Device::CreateRenderTargetView",
        )?;

        Ok(Self {
            size,
            interop: interop.clone(),
            target: target.clone(),
            texture,
            mutex,
            render_target_view,
            handle,
            properties,
            imported: RefCell::new(None),
            last_present: RefCell::new(None),
            disposals: disposals.clone(),
        })
    }

    fn dispose_async(&self) -> JoinHandle<()> {
        let last_present = self.last_present.borrow().clone();
        let render_target_view = self.render_target_view.clone();
        let mutex = self.mutex.clone();
        let texture = self.texture.clone();
        start(async move {
            if let Some(last_present) = last_present {
                // Ignore
                let _ = ServerJobTaskFuture::new(last_present).await;
            }

            drop(render_target_view);
            drop(mutex);
            drop(texture);
        })
    }
}

impl ISwapchainImage for D3D11SwapchainImage {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn last_present(&self) -> Option<SwapchainImagePresentStatus> {
        present_status(self.last_present.borrow().as_ref())
    }

    fn begin_draw(&self) {
        // Upstream ignores the result.
        let _ = self.mutex.acquire_sync(0, i32::MAX as u32);
    }

    fn present(&self) {
        // Upstream ignores the result.
        let _ = self.mutex.release_sync(1);
        if self.imported.borrow().is_none() {
            let imported = self.interop.import_image(
                Rc::new(PlatformHandle::new(
                    self.handle,
                    Some(KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE),
                )),
                self.properties.clone(),
            );
            *self.imported.borrow_mut() = Some(imported);
        }
        let imported = self.imported.borrow().clone().expect("the imported texture");
        *self.last_present.borrow_mut() = Some(self.target.update_with_keyed_mutex_async(&*imported, 1, 0));
    }

    fn dispose(&self) {
        let disposal = self.dispose_async();
        self.disposals.borrow_mut().push(disposal);
    }
}
