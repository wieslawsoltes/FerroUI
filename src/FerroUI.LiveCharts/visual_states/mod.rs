//! Layered property setters and imperative behavior on registered visual values.
//!
//! Definitions may be shared by a series. Snapshots and activation order live
//! on each `Animatable`, so applying a state never changes another point.

mod drawn_property_setter;
mod state_behavior;
mod visual_states_dictionary;

pub use drawn_property_setter::DrawnPropertySetter;
pub use state_behavior::{IStateBehavior, StateBehavior, StateCallback};
pub use visual_states_dictionary::{
    DrawnPropertiesDictionary, StateError, StateOrigin, VisualStatesDictionary,
};

#[cfg(test)]
mod tests;
