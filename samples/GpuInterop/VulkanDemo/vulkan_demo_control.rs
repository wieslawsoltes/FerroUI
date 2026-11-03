//! Port of `VulkanDemo/VulkanDemoControl.cs`: the demo that draws the teapot with Vulkan into
//! images it shares with the compositor.

use super::vulkan_content::VulkanContent;
use super::vulkan_context::VulkanContext;
use super::vulkan_swapchain::VulkanSwapchain;
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

/// The objects the demo draws with.
pub(super) struct VulkanResources<TSwapchain> {
    pub context: Rc<VulkanContext>,
    pub swapchain: TSwapchain,
    pub content: RefCell<VulkanContent>,
}

impl VulkanResources<VulkanSwapchain> {
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
pub struct VulkanDemoControl {
    base: DrawingSurfaceDemoBase,
    resources: RefCell<Option<VulkanResources<VulkanSwapchain>>>,
}

ferro_class!(VulkanDemoControl: DrawingSurfaceDemoBase);
ferro_class_info!(VulkanDemoControl { new: VulkanDemoControl::new });
ferro_impl_classes!(
    VulkanDemoControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawingSurfaceDemoBaseImpl for VulkanDemoControl {
    fn supports_disco(_this: &Self) -> bool {
        true
    }

    fn initialize_graphics_resources(
        this: &Self,
        _compositor: &Rc<Compositor>,
        composition_drawing_surface: &CompositionDrawingSurface,
        gpu_interop: &Rc<dyn ICompositionGpuInterop>,
    ) -> (bool, String) {
        let (context, info) = VulkanContext::try_create(&**gpu_interop, false);
        let Some(context) = context else {
            return (false, info);
        };
        match VulkanContent::new(&context) {
            Ok(content) => {
                *this.resources.borrow_mut() = Some(VulkanResources {
                    swapchain: VulkanSwapchain::new(&context, gpu_interop, composition_drawing_surface),
                    context,
                    content: RefCell::new(content),
                });
                (true, info)
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
        /*
        var commandBuffer = _resources.Context.Pool.CreateCommandBuffer();
        commandBuffer.BeginRecording();
        image.TransitionLayout(commandBuffer.InternalHandle, ImageLayout.TransferDstOptimal, AccessFlags.None);

        var range = new ImageSubresourceRange
        {
            AspectMask = ImageAspectFlags.ColorBit,
            LayerCount = 1,
            LevelCount = 1,
            BaseArrayLayer = 0,
            BaseMipLevel = 0
        };
        var color = new ClearColorValue
        {
            Float32_0 = 1, Float32_1 = 0, Float32_2 = 0, Float32_3 = 1
        };
        _resources.Context.Api.CmdClearColorImage(commandBuffer.InternalHandle, image.InternalHandle.Value, ImageLayout.TransferDstOptimal,
            &color, 1, &range);
        commandBuffer.Submit();*/
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

impl VulkanDemoControl {
    pub fn construct() -> Self {
        Self { base: DrawingSurfaceDemoBase::construct(), resources: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
