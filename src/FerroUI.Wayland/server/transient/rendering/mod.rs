//! The render surfaces of the worker's surfaces (the directory
//! `Server/Transient/Rendering` of the reference): frames in shared memory,
//! and EGL over a `wl_egl_window`.
//!
//! The dmabuf swapchain (`WaylandEglDisplay.cs`,
//! `WaylandEglDmaBufPlatformGraphics.cs`, `WaylandEglDmaBufSurface.cs`,
//! `WaylandDmabufFeedback.cs`) belongs to stage 3 of
//! `docs/porting/wayland-platform.md`.

pub mod i_wayland_framebuffer_surface;
pub mod wayland_egl_wsi_platform_graphics;
pub mod wayland_egl_wsi_surface;
pub mod wayland_framebuffer;
