//! The behaviors, the trigger and the action that draw into a render target bitmap
//! (`RenderTarget/`), with their contracts (`RenderTarget/Contract/`).

mod contract;
mod render_render_target_bitmap_action;
mod render_target_bitmap_behavior;
mod render_target_bitmap_trigger;
mod static_render_target_bitmap_behavior;

pub use contract::{IRenderTargetBitmapRenderHost, IRenderTargetBitmapRenderer, IRenderTargetBitmapSimpleRenderer};
pub use render_render_target_bitmap_action::RenderRenderTargetBitmapAction;
pub use render_target_bitmap_behavior::RenderTargetBitmapBehavior;
pub use render_target_bitmap_trigger::RenderTargetBitmapTrigger;
pub use static_render_target_bitmap_behavior::StaticRenderTargetBitmapBehavior;
