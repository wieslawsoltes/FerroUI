//! Port of the `Compiler/` directory: the backend-independent compiler driver.

mod xaml_compiler;
mod xaml_imperative_compiler;

pub use xaml_compiler::*;
pub use xaml_imperative_compiler::*;
