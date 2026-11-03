//! Port of `VulkanDemo/VulkanDmaBufDemoControl.cs`.
//!
//! Renders the spinning teapot with Vulkan and presents it to the EGL/OpenGL compositor as a Linux
//! dma-buf, exercising the EGL external objects feature of the OpenGL crate (EGL_EXT_image_dma_buf_import).
//! Run with `--dmabuf` so the X11 backend is configured to use the EGL compositor.

use super::vulkan_content::VulkanContent;
use super::vulkan_context::VulkanContext;
use super::vulkan_demo_control::VulkanResources;
use super::vulkan_dma_buf_swapchain::VulkanDmaBufSwapchain;
use crate::drawing_surface_demo_base::{DrawingSurfaceDemoBase, DrawingSurfaceDemoBaseImpl};
use crate::task_support::{join, start};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::composition::{CompositionDrawingSurface, Compositor, ICompositionGpuInterop};
use ferroui_base::threading::JoinHandle;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, PixelSize, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::ControlImpl;
use std::cell::RefCell;
use std::rc::Rc;

impl VulkanResources<VulkanDmaBufSwapchain> {
    fn dispose_async(self) -> JoinHandle<()> {
        self.context.pool().free_used_command_buffers();
        self.content.borrow_mut().dispose();
        let swapchain_disposal = self.swapchain.dispose_async();
        let context = self.context;
        start(async move {
            let _ = join(swapchain_disposal).await;
            context.dispose();
        })
    }
}

#[repr(C)]
pub struct VulkanDmaBufDemoControl {
    base: DrawingSurfaceDemoBase,
    resources: RefCell<Option<VulkanResources<VulkanDmaBufSwapchain>>>,
}

ferro_class!(VulkanDmaBufDemoControl: DrawingSurfaceDemoBase);
ferro_class_info!(VulkanDmaBufDemoControl { new: VulkanDmaBufDemoControl::new });
ferro_impl_classes!(
    VulkanDmaBufDemoControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawingSurfaceDemoBaseImpl for VulkanDmaBufDemoControl {
    fn supports_disco(_this: &Self) -> bool {
        true
    }

    fn initialize_graphics_resources(
        this: &Self,
        _compositor: &Rc<Compositor>,
        composition_drawing_surface: &CompositionDrawingSurface,
        gpu_interop: &Rc<dyn ICompositionGpuInterop>,
    ) -> (bool, String) {
        if !cfg!(target_os = "linux") {
            return (false, "The dma-buf demo is only supported on Linux".to_string());
        }

        let (context, info) = VulkanContext::try_create(&**gpu_interop, true);
        let Some(context) = context else {
            return (false, info);
        };
        match VulkanContent::new(&context) {
            Ok(content) => {
                *this.resources.borrow_mut() = Some(VulkanResources {
                    swapchain: VulkanDmaBufSwapchain::new(&context, gpu_interop, composition_drawing_surface),
                    context,
                    content: RefCell::new(content),
                });
                (true, format!("dma-buf import via EglExternalObjectsFeature\n{info}"))
            }
            Err(e) => (false, e.to_string()),
        }
    }

    fn free_graphics_resources(this: &Self) {
        let resources = this.resources.borrow_mut().take();
        if let Some(resources) = resources {
            drop(resources.dispose_async());
        }
    }

    fn render_frame(this: &Self, pixel_size: PixelSize) {
        let resources = this.resources.borrow();
        let Some(resources) = resources.as_ref() else {
            return;
        };
        let (session, image) = resources.swapchain.begin_draw(pixel_size);
        resources.content.borrow_mut().render(
            image.image(),
            f64::from(this.yaw()),
            f64::from(this.pitch()),
            f64::from(this.roll()),
            f64::from(this.disco()),
        );
        session.dispose();
    }
}

impl VulkanDmaBufDemoControl {
    pub fn construct() -> Self {
        Self { base: DrawingSurfaceDemoBase::construct(), resources: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
