//! Tests ported from the upstream `tests/XamlParserTests` project (the ones that do not need to
//! run generated code) plus AST level tests of the transformers on the fake type system.

#![allow(clippy::cloned_ref_to_slice_refs)]

mod emit_tests;
mod helpers;
mod parser_tests;
mod test_xaml_language;
mod transformer_tests;
mod type_system_tests;
mod whitespace_tests;
