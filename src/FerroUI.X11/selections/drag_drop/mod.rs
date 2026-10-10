//! Drag and drop between X clients through the XDND protocol, version 5
//! (the port of the `Selections/DragDrop` directory).
//!
//! Specs: <https://www.freedesktop.org/wiki/Specifications/XDND/>

pub mod drag_drop_data_provider;
pub mod drag_drop_data_reader;
pub mod drag_drop_data_transfer;
pub mod drag_drop_data_transfer_item;
pub mod drag_drop_timeout_manager;
pub mod i_xdnd_window;
pub mod synchronous_x_event_waiter;
pub mod x11_drag_source;
pub mod x11_drop_target;
pub mod xdnd_action_helper;
pub mod xdnd_constants;

pub use i_xdnd_window::IXdndWindow;
pub use x11_drag_source::X11DragSource;
pub use x11_drop_target::X11DropTarget;
