//! ferroui-markup
//!
//! Markup support: the binding created from markup and the grammars of
//! selectors, property paths and container queries.

#[doc(hidden)]
pub mod binding_opcodes;
#[doc(hidden)]
pub mod compiled_operands;
#[doc(hidden)]
pub mod construction_opcodes;
pub mod data;
pub mod markup;
#[doc(hidden)]
pub mod path_opcodes;
#[doc(hidden)]
pub mod resource_opcodes;
#[doc(hidden)]
pub mod style_opcodes;

#[cfg(test)]
mod tests;
