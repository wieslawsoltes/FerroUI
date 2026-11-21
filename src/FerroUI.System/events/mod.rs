//! Multicast events: the lists of handlers that tolerate changes while they are invoked, and the
//! event built on them.

mod event;
mod handler_list;

pub use event::Event;
pub use handler_list::{HandlerIter, HandlerList, HandlerSnapshot};
