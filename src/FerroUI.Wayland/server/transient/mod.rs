//! What is bound to one connection to the compositor (the directory
//! `Server/Transient` of the reference): the globals, the outputs, the
//! seats, the cursor theme and the render surfaces. All of it is made again
//! for a new connection.

pub mod wayland_input_dispatcher;
pub mod wayland_outputs_tracker;

#[cfg(target_os = "linux")]
pub mod rendering;
#[cfg(target_os = "linux")]
pub mod wayland_cursor_manager;
#[cfg(target_os = "linux")]
pub mod wayland_globals;
#[cfg(target_os = "linux")]
mod wayland_input_dispatcher_keyboard;
