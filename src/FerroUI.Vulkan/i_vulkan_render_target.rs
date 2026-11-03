use crate::VulkanImageInfo;
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// The Vulkan render target of a surface.
///
/// The contract is implemented by this crate and by surfaces that provide
/// their own render target.
pub trait IVulkanRenderTarget: IPlatformRenderSurfaceRenderTarget {
    /// Starts rendering a frame.
    ///
    /// # Panics
    /// Panics when the image to render to cannot be created.
    fn begin_draw(&self) -> Rc<dyn IVulkanRenderSession>;

    /// Starts a frame with its negotiated size and scale. Window systems with
    /// asynchronous configuration can defer drawing until a configure arrives.
    fn try_begin_draw(&self, _scene: &ferroui_base::platform::RenderTargetSceneInfo)
        -> Result<Rc<dyn IVulkanRenderSession>, ferroui_base::platform::RenderTargetError> {
        Ok(self.begin_draw())
    }

    /// Releases the render target.
    fn dispose(&self);
}

/// One frame being rendered with Vulkan. Disposing the session presents the
/// frame.
pub trait IVulkanRenderSession {
    /// The scaling from logical units to device pixels.
    fn scaling(&self) -> f64;

    /// The size of the image in device pixels.
    fn size(&self) -> PixelSize;

    /// Whether the first row of the image is its top row.
    fn is_y_flipped(&self) -> bool;

    /// The image to render to.
    fn image_info(&self) -> VulkanImageInfo;

    /// Whether the channels of the image are in the order red, green, blue,
    /// alpha (and not blue, green, red, alpha).
    fn is_rgba(&self) -> bool;

    /// Finishes and presents the frame.
    fn dispose(&self);
}
