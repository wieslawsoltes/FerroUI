//! The render surfaces of the worker's surfaces (the directory
//! `Server/Transient/Rendering` of the reference): frames in shared memory,
//! EGL over a `wl_egl_window`, and the dmabuf swapchain (EGL over a GBM
//! device, with buffers handed to the compositor through
//! `zwp_linux_dmabuf_v1`).
//!
//! The decoding of the dmabuf feedback is plain logic and exists on every
//! system; the rest talks to a compositor and exists on Linux only.

pub mod wayland_dmabuf_feedback;

#[cfg(target_os = "linux")]
pub mod i_wayland_framebuffer_surface;
#[cfg(target_os = "linux")]
pub mod wayland_egl_display;
#[cfg(target_os = "linux")]
pub mod wayland_egl_dma_buf_platform_graphics;
#[cfg(target_os = "linux")]
pub mod wayland_egl_dma_buf_surface;
#[cfg(target_os = "linux")]
pub mod wayland_egl_wsi_platform_graphics;
#[cfg(target_os = "linux")]
pub mod wayland_egl_wsi_surface;
#[cfg(target_os = "linux")]
pub mod wayland_framebuffer;

#[cfg(target_os = "linux")]
pub mod wayland_vulkan;
