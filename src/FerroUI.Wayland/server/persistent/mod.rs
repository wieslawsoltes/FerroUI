//! The state of the backend that outlives a connection to the compositor
//! (the directory `Server/Persistent` of the reference): surfaces and cursors
//! that are sent again to a compositor that restarted, and the contracts
//! between them and the UI thread.

pub mod decoration_mode;
pub mod i_w_surface_event_sink;
pub mod w_surface;
pub mod xdg_configure_batch;
pub mod xdg_popup_configure_batch;

#[cfg(target_os = "linux")]
pub mod i_persistent_object;
#[cfg(target_os = "linux")]
pub mod i_w_surface;
#[cfg(target_os = "linux")]
pub mod i_w_xdg_top_level;
#[cfg(target_os = "linux")]
pub mod i_wayland_cursor;
#[cfg(target_os = "linux")]
pub mod wayland_bitmap_cursor;
#[cfg(target_os = "linux")]
pub mod wayland_cursor;
#[cfg(target_os = "linux")]
pub mod wayland_input_event_cookie;
#[cfg(target_os = "linux")]
pub mod xdg_popup_positioner_params;
