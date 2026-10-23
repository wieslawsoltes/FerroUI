//! todo-mvc
//!
//! TodoMVC (<https://todomvc.com>, the specification of its applications)
//! written with the declarative API of `ferroui-view`: components that run
//! again, with their state in hooks, and the router of single-window
//! applications, whose routes are the filters of the list. It runs in the
//! WinUI 3 theme.
//!
//! - [`model`]: the todos, the operations on the list, its counts and its
//!   filters, without user interface.
//! - [`view`]: the components ([`App`] and those below it).
//! - [`run`]: the application in a window, and its picture with
//!   `FERROUI_TODOMVC_SHOTS=<directory>`.
//!
//! `cargo run -p todo-mvc`.

pub mod model;
mod shots;
pub mod view;

pub use shots::{capture_if_asked, run};
pub use view::{app, App};

#[cfg(test)]
mod tests;
