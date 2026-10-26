//! Deterministic catalog generation and opt-in source diagnostics for ProTranslate.
//!
//! A build script explicitly names its catalog files. Generated providers use static
//! tables and typed accessors; no runtime resource discovery is required.

mod compiler;
mod source;

pub use compiler::*;
pub use source::*;

#[cfg(test)]
mod tests;
