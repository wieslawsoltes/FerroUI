//! ferroui-xaml-interactions-responsive
//!
//! Behaviors that switch style classes by the size of a control, ported from the
//! `Xaml.Behaviors.Interactions.Responsive` project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - [`AdaptiveBehavior`] observes the bounds of its control (or of its source control) and adds
//!   or removes the class of each of its [`AdaptiveClassSetter`]s as the width and the height
//!   meet the minimum and maximum of the setter;
//! - [`AspectRatioBehavior`] does the same with the ratio of the width to the height and the
//!   minimum and maximum ratios of its [`AspectRatioClassSetter`]s.
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.Responsive`, mapped into the default
//! XML namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

mod adaptive_behavior;
mod adaptive_class_setter;
mod aspect_ratio_behavior;
mod aspect_ratio_class_setter;
mod markup_types;
mod register_types;
mod rust_paths;

pub use crate::adaptive_behavior::{AdaptiveBehavior, AdaptiveClassSetters};
pub use crate::adaptive_class_setter::AdaptiveClassSetter;
pub use crate::aspect_ratio_behavior::{AspectRatioBehavior, AspectRatioClassSetters};
pub use crate::aspect_ratio_class_setter::AspectRatioClassSetter;
pub use crate::register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
