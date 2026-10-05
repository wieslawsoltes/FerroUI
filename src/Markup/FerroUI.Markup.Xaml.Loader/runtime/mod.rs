//! The run-time back end of the XAML compiler.
//!
//! Where the managed original compiles the transformed AST to IL and runs
//! it, this back end interprets the same transformed AST against run-time
//! type metadata:
//!
//! * [`type_system`] is the type system the transformers run on at run time:
//!   the registered classes, markup metadata and assemblies projected onto
//!   the type system abstraction of `xamlx`, every member with its invoker.
//! * [`interpreter`] evaluates the transformed AST node by node with the
//!   semantics of the IL emitters of the managed original.

pub mod framework;
pub mod interpreter;
pub mod type_system;
pub(crate) mod value_parser;

pub use value_parser::RuntimeCompileTimeValueParser;
