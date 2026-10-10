//! Software rendering through the shared memory extension of the server
//! (the port of the `XShm` directory).

pub mod x11_shm_framebuffer_render_target;
pub mod x11_shm_framebuffer_surface;
pub mod x11_shm_image;

pub use x11_shm_framebuffer_render_target::X11ShmFramebufferRenderTarget;
pub use x11_shm_framebuffer_surface::X11ShmFramebufferSurface;
pub use x11_shm_image::X11ShmImage;
