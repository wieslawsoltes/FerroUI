//! The third fixture crate of the tests of the source scanner (`ferroui-build`,
//! `scanner`): what a crate states next to its declarations, and what decides how the
//! declarations are registered. The list of the classes it registers, the handles and the
//! casts its registration function adds, type aliases, accessors a macro of the crate
//! writes among the members of a type, a property whose accessor is a function of another
//! type, and enumerations whose members are constant expressions.
//! It is read as files and never compiled: most of what it names does not exist.

pub mod keys;
pub mod panel;
mod register_types;

pub use panel::{Marker, Panel, PanelList, Slot};
