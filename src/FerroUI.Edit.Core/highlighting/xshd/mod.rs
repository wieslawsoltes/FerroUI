//! Editable XSHD syntax trees, visitors, XML loading and serialization.
//!
//! Syntax trees retain source order and unresolved references. Compilation is a
//! separate operation so callers can inspect or modify a definition before it
//! becomes an executable highlighting definition.

mod compiler;
mod legacy;
mod loader;
mod model;
mod visitor;
mod writer;

pub use compiler::compile_xshd;
pub use loader::load_xshd;
pub use model::*;
pub use visitor::{XshdVisitor, XshdVisitor as IXshdVisitor, XshdVisitorMut};
pub use writer::{SaveXshdVisitor, XSHD_NAMESPACE};

#[cfg(test)]
mod tests;
