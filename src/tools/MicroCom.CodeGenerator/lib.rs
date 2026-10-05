//! microcom-codegen
//!
//! Rust port of the parts of the MicroCom code generator
//! (<https://github.com/kekekeks/MicroCom>) needed for the native interop IDL (`frn.idl`):
//!
//! * [`parse`] turns IDL text into an [`ast::Idl`];
//! * [`cpp::generate`] emits the C++ header (same output as MicroCom's
//!   C++ generator, i.e. `ferro-native.h`);
//! * [`rust::generate`] emits Rust bindings on top of `ferroui-microcom`.

pub mod ast;
pub mod cpp;
mod parser;
pub mod rust;

pub use parser::{parse, ParseError};
