// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Build-time adapter for the shared declaration model. It links no runtime or UI.
//!
//! The semantic compiler remains responsible for activation, registrations, ignores
//! and subtype selection. This adapter binds declared members to typed Rust calls;
//! it does not infer abstractness, visibility, inheritance or class casts from names.
mod dependency_symbols;
mod generation_attributes;
mod generic_metadata;
mod metadata_bindings;
mod receiver_projection;
mod semantic_symbols;
mod source_declarations;
pub use dependency_symbols::*;
pub use generation_attributes::*;
pub use generic_metadata::instantiate_generic_metadata;
pub use metadata_bindings::*;
pub use receiver_projection::*;
pub use semantic_symbols::*;
pub use source_declarations::*;
#[cfg(test)]
mod compiler_fixture_input;
#[cfg(test)]
mod dependency_symbol_tests;
#[cfg(test)]
mod generation_attribute_tests;
#[cfg(test)]
mod generic_metadata_tests;
#[cfg(test)]
mod native_contract_tests;
#[cfg(test)]
mod native_model_schema_tests;
#[cfg(test)]
mod semantic_batch_tests;
#[cfg(test)]
mod semantic_symbol_tests;
#[cfg(test)]
mod source_declaration_tests;
#[cfg(test)]
mod tests;
