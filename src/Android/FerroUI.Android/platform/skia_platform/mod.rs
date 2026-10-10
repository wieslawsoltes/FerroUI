//! The top-level and its surfaces.

pub(crate) mod android_framebuffer;

#[cfg(target_os = "android")]
mod framebuffer_manager;
#[cfg(target_os = "android")]
mod invalidation_aware_surface_view;
#[cfg(target_os = "android")]
mod top_level_impl;

#[cfg(target_os = "android")]
pub(crate) use invalidation_aware_surface_view::SurfaceProperties;
#[cfg(target_os = "android")]
pub use top_level_impl::TopLevelImpl;
