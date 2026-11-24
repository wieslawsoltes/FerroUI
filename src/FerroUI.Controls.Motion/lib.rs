//! ferroui-controls-motion
//!
//! The motion controls of WinUI on the composition API. An application that does not use them
//! links nothing of this crate.
//!
//! The implicit transitions of an element are attached properties of [`ImplicitTransitions`]
//! with markup aliases on their WinUI names, so `<Rectangle.OpacityTransition>` costs nothing to
//! an application that does not depend on this crate.
//!
//! [`ParallaxView`] moves its child at a different speed than a scrolling source. The motion is
//! a set of expression animations compiled once on the composition visual of the child: while
//! the source scrolls, only the values they read change.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_motion;

mod implicit_transitions;
mod parallax_expressions;
mod parallax_view;
mod register_types;

pub use implicit_transitions::{ImplicitTransitions, ScalarTransition, Vector3Transition, Vector3TransitionComponents};
pub use parallax_view::{ParallaxSourceOffsetKind, ParallaxView};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod implicit_transitions_tests;
#[cfg(test)]
mod parallax_view_tests;
