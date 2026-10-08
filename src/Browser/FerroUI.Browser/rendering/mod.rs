//! Rendering to the canvas of a view: the surface, its render targets (a
//! WebGL context or a framebuffer copied to a 2D canvas), the render timer
//! driven by the animation frames of the page or of a render worker, the
//! render worker, and what the thread of the user interface and a render
//! thread share about a canvas.
//!
//! By who holds what (`docs/porting/browser-render-worker.md`, section 4):
//!
//! - [`BrowserSurfaceShared`] is what both threads know about a canvas, and
//!   [`BrowserRenderSurface`] the render surface over it that the compositor
//!   is handed. Neither holds an object of a thread.
//! - [`BrowserRenderTarget`] ([`BrowserWebGlRenderTarget`],
//!   [`BrowserSoftwareRenderTarget`]) is the render target of a canvas on
//!   the thread that draws to it, kept in a table of that thread
//!   ([`get_render_target`]).
//! - [`RenderTargetBrowserSurface`] is the surface of a view on the thread
//!   of the user interface.
//!
//! Without a render thread the three live on the one thread of the page.
//!
//! With one ([`BrowserSharedRenderLoop::start_render_thread`], which the
//! platform calls in a module built with threads) the compositor of a view
//! is confined to it: the frame loop ticks there, the canvas of a view is
//! transferred to its worker, and the thread of the user interface commits
//! batches and waits. [`RenderStatistics`] is where the page reads which
//! thread drew its frames.

mod browser_render_surface;
mod browser_render_timer;
mod browser_shared_render_loop;
mod browser_software_render_target;
mod browser_surface;
mod browser_surface_shared;
mod browser_web_gl_render_target;
mod render_statistics;
mod render_target_browser_surface;
mod render_worker;
mod web_render_target;

pub use browser_render_surface::BrowserRenderSurface;
pub use browser_render_timer::BrowserRenderTimer;
pub use browser_shared_render_loop::BrowserSharedRenderLoop;
pub use browser_software_render_target::BrowserSoftwareRenderTarget;
pub use browser_surface::BrowserSurface;
pub use browser_surface_shared::BrowserSurfaceShared;
pub use browser_web_gl_render_target::{BrowserWebGlRenderTarget, WebGlContext};
pub use render_statistics::RenderStatistics;
pub use render_target_browser_surface::RenderTargetBrowserSurface;
pub use render_worker::RenderWorker;
pub use web_render_target::{
    get_render_target, initialize_worker, remove_render_target, BrowserRenderTarget, PENDING_RENDER_THREAD,
};
