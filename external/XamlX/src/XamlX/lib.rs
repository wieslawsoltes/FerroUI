//! Rust port of the backend-independent part of XamlX, the XAML compiler library by
//! Nikita Tsukanov (MIT, see `external/XamlX/NOTICE.md`).
//!
//! The crate mirrors the upstream layout module for module:
//!
//! | Module | Upstream | Contents |
//! |---|---|---|
//! | [`ast`] | `Ast/` | AST nodes, visitors, type/property references, intrinsics |
//! | [`parsers`] | `Parsers/` | XML based XAML parser and the markup extension parser |
//! | [`type_system`] | `TypeSystem/` | the type system abstraction and its helpers |
//! | [`transform`] | `Transform/` | transformation context, configuration and all AST transformers |
//! | [`emit`] | `Emit/` | backend-agnostic emit contracts |
//! | [`compiler`] | `Compiler/` | the backend-independent compiler driver |
//! | [`diagnostics`] | `Diagnostics/` | diagnostics and their severity |
//! | [`exceptions`] | `Exceptions.cs` | the error type |
//!
//! The IL backend (`IL/`) is not part of the port. Upstream AST nodes implement the IL
//! emitting interfaces directly; here the nodes carry only their data, and a backend supplies
//! node emitters ([`emit::IXamlAstNodeEmitter`]) for them.
//!
//! # Conventions
//!
//! * C# interfaces keep their `I` prefix as trait names and are used through `Rc<dyn ...>`
//!   handles; a C# property named `Type` becomes `type_`.
//! * Nodes are mutable through `Cell`/`RefCell` fields, which keeps the upstream in-place
//!   visitor semantics and node identity.
//! * Exceptions become [`XamlResult`] values carrying an [`XamlError`].

#![forbid(unsafe_code)]
// The port keeps the upstream shapes (tuple lists, callback signatures, module names).
#![allow(clippy::type_complexity, clippy::module_inception)]

pub mod ast;
pub mod compiler;
pub mod diagnostics;
pub mod emit;
pub mod exceptions;
pub mod extensions;
pub mod parsers;
pub mod transform;
pub mod type_system;
pub mod xaml_namespaces;

/// An in-memory type system for tests of transformers and emitter backends.
#[cfg(any(test, feature = "testing"))]
pub mod testing;

#[cfg(test)]
mod tests;

pub use diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity, XamlXWellKnownDiagnosticCodes};
pub use exceptions::{XamlError, XamlParseException, XamlResult, XamlTypeSystemException};
pub use xaml_namespaces::XamlNamespaces;
