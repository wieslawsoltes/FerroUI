//! The assembly attributes of the reference (`Properties/`).
//!
//! `AssemblyInfo.cs` states the XML namespace definitions of the crate, which are in
//! `register_types.rs` (`ASSEMBLY`), and grants its unit tests access to internal members, which
//! the in-crate tests have without it; `AnimationTypeForwards.cs` forwards the animation types of
//! the animations crate to this crate ([`animation_type_forwards`]).

mod animation_type_forwards;

pub use animation_type_forwards::*;
