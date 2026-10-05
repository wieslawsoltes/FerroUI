//! Port of the `Emit/` directory: the backend-agnostic emit abstractions.
//!
//! `TBackendEmitter` is the backend's code generator *handle* (a cheaply clonable reference
//! type, as in C#) and `TEmitResult` is what emitting a node yields. Both stay type parameters
//! of the contexts, emitters and mappings, while AST node types stay non-generic: a backend
//! provides node emitters ([`IXamlAstNodeEmitter`]) for the built-in nodes, and nodes that can
//! emit themselves expose [`IXamlAstEmitableNode`] through `IXamlAstNode::query_interface`.

mod xaml_emit_context;
mod xaml_emit_context_with_locals;
mod xaml_language_emit_mappings;
mod xaml_runtime_context;

pub use xaml_emit_context::*;
pub use xaml_emit_context_with_locals::*;
pub use xaml_language_emit_mappings::*;
pub use xaml_runtime_context::*;
