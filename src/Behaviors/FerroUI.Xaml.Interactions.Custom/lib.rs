//! ferroui-xaml-interactions-custom
//!
//! The custom behaviors, triggers and actions of FerroUI, ported from the
//! `Xaml.Behaviors.Interactions.Custom` project of Xaml.Behaviors (see `NOTICE.md`): behaviors for
//! specific controls (buttons, items controls, split views, windows, scroll viewers, text boxes,
//! ...), input, gestures and focus, validation, commands executed on events, dialogs and
//! notifications, animations and transitions, and services of the top level.
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.Custom` (and
//! `FerroUI.Xaml.Interactions.Custom.Converters`), mapped into the default XML namespace of the
//! framework: documents name its types without a prefix once [`register_types`] ran.
//!
//! The crate is organised in four parts, each with the folders of the reference it holds
//! (`part_a.rs` to `part_d.rs`); the folders keep the names of the reference in snake case.

pub mod converters;
mod part_a;
mod part_b;
mod part_c;
mod part_d;
mod register_types;

pub use crate::part_a::*;
pub use crate::part_b::*;
pub use crate::part_c::*;
pub use crate::part_d::*;
pub use crate::register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
