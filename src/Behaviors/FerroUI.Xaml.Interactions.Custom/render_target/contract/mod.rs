//! The contracts of the render target bitmap behaviors (`RenderTarget/Contract/`).

mod i_render_target_bitmap_render_host;
mod i_render_target_bitmap_renderer;
mod i_render_target_bitmap_simple_renderer;

pub use i_render_target_bitmap_render_host::IRenderTargetBitmapRenderHost;
pub use i_render_target_bitmap_renderer::IRenderTargetBitmapRenderer;
pub use i_render_target_bitmap_simple_renderer::IRenderTargetBitmapSimpleRenderer;
