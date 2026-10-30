// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiler semantic passes for source JSON registrations; no runtime model registry.
mod generation_model;
mod generation_model_builder;
mod rust_bindings;
mod source_emitter;
mod symbols;
pub use generation_model::*;
pub use generation_model_builder::{build, full_name};
pub use rust_bindings::*;
pub use source_emitter::emit_rust;
pub use symbols::*;
#[cfg(test)]
mod tests;
