//! UI Automation: the provider side of the accessibility of Windows (the
//! port of the reference's automation project of the Windows backend,
//! which is internal to the backend there too).
//!
//! A window answers `WM_GETOBJECT` with the provider of its root; a tree of
//! nodes over the automation peers of the controls answers what a client
//! (a screen reader, a test tool) asks, and raises the events of the peers.
//! The design is in `docs/porting/win32-platform.md`, section 13.

pub mod interop;
pub mod marshalling;
#[cfg(windows)]
#[doc(hidden)]
pub mod testing;

mod automation_event_sink;
mod automation_node;
mod automation_node_expand_collapse;
mod automation_node_range_value;
mod automation_node_scroll;
mod automation_node_selection;
mod automation_node_text;
mod automation_node_toggle;
mod automation_node_value;
mod interop_automation_node;
mod root_automation_node;
mod text_pattern_marshalling;
mod text_range_node;

pub(crate) use automation_event_sink::Win32AutomationEventSink;
#[allow(unused_imports)] // The window uses them on Windows.
pub(crate) use automation_node::{AutomationNode, NodeKind};
#[allow(unused_imports)]
pub(crate) use interop_automation_node::InteropAutomationNode;
#[allow(unused_imports)]
pub(crate) use root_automation_node::RootAutomationNode;

#[cfg(test)]
mod automation_node_tests;
#[cfg(test)]
mod text_pattern_tests;
