//! Rendering to the canvas of a view: the surface, its render targets (a
//! WebGL context or a framebuffer copied to a 2D canvas) and the render
//! timer driven by the animation frames of the page.

mod browser_render_timer;
mod browser_shared_render_loop;
mod browser_software_render_target;
mod browser_surface;
mod browser_web_gl_render_target;
mod render_target_browser_surface;
mod web_render_target;

pub use browser_render_timer::BrowserRenderTimer;
pub use browser_shared_render_loop::BrowserSharedRenderLoop;
pub use browser_software_render_target::BrowserSoftwareRenderTarget;
pub use browser_surface::BrowserSurface;
pub use browser_web_gl_render_target::{BrowserWebGlRenderTarget, WebGlContext};
pub use render_target_browser_surface::RenderTargetBrowserSurface;
pub use web_render_target::{get_render_target, initialize_worker, BrowserRenderTarget, CanvasSize};
