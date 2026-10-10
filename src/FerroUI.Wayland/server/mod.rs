//! The worker side of the backend (the directory `Server` of the reference):
//! what runs on the thread that owns the connection to the compositor, and
//! the handles the UI thread reaches it through.

pub mod persistent;
pub mod server_signaler;
pub mod transient;
pub mod wayland_dispatch_priority;
pub mod wayland_marshallers;
pub mod wayland_platform_graphics;
pub mod wayland_worker_render_timer;

#[cfg(target_os = "linux")]
pub mod interop;
#[cfg(target_os = "linux")]
pub mod wayland_worker;
#[cfg(target_os = "linux")]
pub mod wayland_worker_client;
