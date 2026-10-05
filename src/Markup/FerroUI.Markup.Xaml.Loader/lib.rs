//! ferroui-markup-xaml-loader
//!
//! The XAML language of the framework: the compiler extensions (language definition,
//! well-known types, AST nodes, transformers) that turn a parsed XAML document into the typed
//! AST the back ends consume. The crate builds on `xamlx` and mirrors the layout of the
//! upstream loader project module for module:
//!
//! | Module | Upstream | Contents |
//! |---|---|---|
//! | [`compiler_extensions`] | `CompilerExtensions/` | language, well-known types, nodes, transformers |
//! | [`parsers`] | `Markup.Xaml/Parsers` | parsers shared with the markup runtime |
//! | [`testing`] | (none) | the in-memory framework fixture for unit tests (feature `testing`) |
//!
//! There is no IL back end. Types that generate IL upstream carry their data and document the
//! semantics a back end has to implement; see the table in [`compiler_extensions`].

#![forbid(unsafe_code)]
// The port keeps the upstream shapes (constructor argument lists, callback signatures).
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod compiler_extensions;
pub mod parsers;

/// The run-time back end: the run-time type system and the AST interpreter.
#[cfg(feature = "runtime")]
pub mod runtime;

/// The emitter of Rust source: the build-time back end over the same transformed AST.
#[cfg(any(feature = "emitter", all(test, feature = "runtime")))]
pub mod rust_emitter;

#[cfg(feature = "runtime")]
mod ferro_runtime_xaml_loader;
#[cfg(feature = "runtime")]
mod ferro_xaml_il_runtime_compiler;

#[cfg(feature = "runtime")]
pub use ferro_runtime_xaml_loader::{DocumentGroup, FerroRuntimeXamlLoader};
#[cfg(feature = "runtime")]
pub use ferro_xaml_il_runtime_compiler::FerroXamlIlRuntimeCompiler;

/// The in-memory framework type system used by unit tests of the compiler extensions.
#[cfg(any(test, feature = "testing"))]
pub mod testing;
