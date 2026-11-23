//! ferroui-controls-motion
//!
//! The motion controls of WinUI on the composition API. An application that does not use them
//! links nothing of this crate.
//!
//! [`ParallaxView`] moves its child at a different speed than a scrolling source. The motion is
//! a set of expression animations compiled once on the composition visual of the child: while
//! the source scrolls, only the values they read change.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_motion;

mod parallax_expressions;
mod parallax_view;
mod register_types;

pub use parallax_view::{ParallaxSourceOffsetKind, ParallaxView};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod parallax_view_tests;
