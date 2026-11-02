use crate::interop::{VulkanDisplay, VulkanImage};
use crate::{
    IVulkanKhrSurfacePlatformSurface, IVulkanPlatformGraphicsContext, IVulkanRenderSession, IVulkanRenderTarget,
    VulkanError, VulkanImageInfo,
};
use ash::vk;
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::PlatformRenderTargetState;
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The render target of a surface with a swap chain: a frame is rendered to
/// an image of the device and copied to the next image of the swap chain
/// when its session ends.
pub(crate) struct VulkanKhrRenderTarget {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    display: RefCell<Option<Rc<VulkanDisplay>>>,
    image: RefCell<Option<Rc<VulkanImage>>>,
    platform_surface: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    format: vk::Format,
    is_rgba: bool,
}

/// Whether a surface of the given format has its channels in the order red,
/// green, blue, alpha, and the format of the image a frame is rendered to.
pub(crate) fn render_format(surface_format: vk::Format) -> (bool, vk::Format) {
    let is_rgba = surface_format.as_raw() >= vk::Format::R8G8B8A8_UNORM.as_raw()
        && surface_format.as_raw() <= vk::Format::R8G8B8A8_SRGB.as_raw();
    // Skia seems to only create surfaces from images with unorm format
    (is_rgba, if is_rgba { vk::Format::R8G8B8A8_UNORM } else { vk::Format::B8G8R8A8_UNORM })
}

impl VulkanKhrRenderTarget {
    pub fn new(
        surface: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
    ) -> Result<Self, VulkanError> {
        let display = VulkanDisplay::create_display(&context, surface.clone())?;
        let (is_rgba, format) = render_format(display.surface_format()?.format);
        Ok(Self {
            context,
            display: RefCell::new(Some(Rc::new(display))),
            image: RefCell::new(None),
            platform_surface: surface,
            format,
            is_rgba,
        })
    }

    #[allow(dead_code)]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    fn display(&self) -> Rc<VulkanDisplay> {
        self.display.borrow().clone().expect("the render target has been disposed")
    }

    fn create_image(&self, display: &VulkanDisplay) -> Result<(), VulkanError> {
        let image = VulkanImage::new(
            self.context.clone(),
            display.command_buffer_pool().clone(),
            self.format,
            display.size(),
            0,
        )?;
        *self.image.borrow_mut() = Some(Rc::new(image));
        Ok(())
    }

    fn destroy_image(&self) {
        let _ = self.context.device_api().device_wait_idle(self.context.device_handle());
        if let Some(image) = self.image.borrow_mut().take() {
            image.dispose();
        }
    }

    fn try_begin_draw(&self, lock: &LockGuard) -> Result<Rc<VulkanImage>, VulkanError> {
        let _ = lock;
        let display = self.display();
        display.command_buffer_pool().free_used_command_buffers();
        let recreated = display.ensure_swapchain_available()?;
        let image = self.image.borrow().clone();
        match image {
            Some(image) if !recreated => {
                image.transition_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::NONE)?;
            }
            _ => {
                self.destroy_image();
                self.create_image(&display)?;
            }
        }
        Ok(self.image.borrow().clone().expect("the image was created above"))
    }
}

/// The lock of the device for the time of a frame. It is released when the
/// session ends, and when beginning or ending the frame fails.
struct LockGuard(Rc<dyn IDisposable>);

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

impl IPlatformRenderSurfaceRenderTarget for VulkanKhrRenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }
}

impl IVulkanRenderTarget for VulkanKhrRenderTarget {
    fn dispose(&self) {
        let _ = self.context.device_api().device_wait_idle(self.context.device_handle());
        self.destroy_image();
        if let Some(display) = self.display.borrow_mut().take() {
            display.dispose();
        }
    }

    fn begin_draw(&self) -> Rc<dyn IVulkanRenderSession> {
        let l = LockGuard(self.context.ensure_current());
        let image = match self.try_begin_draw(&l) {
            Ok(image) => image,
            Err(error) => panic!("{error}"),
        };
        Rc::new(RenderingSession::new(self.display(), image, self.is_rgba, self.platform_surface.scaling(), l))
    }
}

/// One frame of a [`VulkanKhrRenderTarget`].
pub(crate) struct RenderingSession {
    display: Rc<VulkanDisplay>,
    image: Rc<VulkanImage>,
    is_rgba: bool,
    scaling: f64,
    dispose: RefCell<Option<LockGuard>>,
    disposed: Cell<bool>,
}

impl RenderingSession {
    fn new(display: Rc<VulkanDisplay>, image: Rc<VulkanImage>, is_rgba: bool, scaling: f64, dispose: LockGuard) -> Self {
        Self { display, image, is_rgba, scaling, dispose: RefCell::new(Some(dispose)), disposed: Cell::new(false) }
    }

    fn present(&self) -> Result<(), VulkanError> {
        let command_buffer = self.display.start_presentation()?;
        self.display.blit_image_to_current_image(&command_buffer, &self.image);
        self.display.end_presentation(&command_buffer)
    }
}

impl IVulkanRenderSession for RenderingSession {
    fn size(&self) -> PixelSize {
        self.image.size()
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        true
    }

    fn image_info(&self) -> VulkanImageInfo {
        self.image.image_info()
    }

    fn is_rgba(&self) -> bool {
        self.is_rgba
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        // The guard releases the device whether presenting succeeds or not.
        let _lock = self.dispose.borrow_mut().take();
        if let Err(error) = self.present() {
            panic!("{error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_formats_from_rgba_unorm_to_rgba_srgb_render_as_rgba() {
        assert_eq!(render_format(vk::Format::R8G8B8A8_UNORM), (true, vk::Format::R8G8B8A8_UNORM));
        assert_eq!(render_format(vk::Format::R8G8B8A8_SRGB), (true, vk::Format::R8G8B8A8_UNORM));
        assert_eq!(render_format(vk::Format::R8G8B8A8_SNORM), (true, vk::Format::R8G8B8A8_UNORM));
    }

    #[test]
    fn every_other_format_renders_as_bgra() {
        assert_eq!(render_format(vk::Format::B8G8R8A8_UNORM), (false, vk::Format::B8G8R8A8_UNORM));
        assert_eq!(render_format(vk::Format::B8G8R8A8_SRGB), (false, vk::Format::B8G8R8A8_UNORM));
        assert_eq!(render_format(vk::Format::R8G8B8_UNORM), (false, vk::Format::B8G8R8A8_UNORM));
        assert_eq!(render_format(vk::Format::A2B10G10R10_UNORM_PACK32), (false, vk::Format::B8G8R8A8_UNORM));
    }
}
