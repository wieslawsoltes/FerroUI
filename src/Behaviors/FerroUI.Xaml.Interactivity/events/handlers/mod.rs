//! The handlers of the registry of event handlers (`Events/Handlers/`).

mod button_click_event_handler;
mod flyout_event_handler;
mod func_event_handler;
mod menu_item_click_event_handler;

#[allow(unused_imports)]
pub(crate) use button_click_event_handler::ButtonClickEventHandler;
pub(crate) use flyout_event_handler::FlyoutEventHandler;
pub use func_event_handler::{EventArgsValue, EventHandler, FuncAddEventHandler};
#[allow(unused_imports)]
pub(crate) use menu_item_click_event_handler::MenuItemClickEventHandler;
