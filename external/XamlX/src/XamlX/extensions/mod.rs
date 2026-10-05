//! Port of the `Extensions/` directory.
//!
//! Upstream `TypeExtensions.cs` only contains `System.Type` reflection helpers used by the
//! reflection-emit type system, which is not part of this port. What lives here instead is the
//! glue Rust needs for the C# `obj is ISomeInterface<T>` checks on open sets of types.

mod query_interface;

pub use query_interface::*;
